//! HTTP API handlers and shared app state.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::render::RenderConfig;
use crate::search::{SearchError, SearchMatch, SearchMode, search_chunk};
use crate::segments::{Segment, ansi_to_segments};
use crate::source::{Metadata, SourceClient, SourceError};

/// Cap on the byte range any single /api/render request may ask for. The client is
/// expected to paginate; this is a defence against an accidental "render the whole 2
/// GiB log in one call." Tunable later if it becomes the bottleneck.
const MAX_RENDER_BYTES: u64 = 16 * 1024 * 1024;

/// How many bytes past `q.end` we fetch so that lines whose start is in
/// `[q.start, q.end)` but whose terminating newline lands past `q.end` can be included
/// in their entirety. Without this, every line that straddles a chunk boundary is
/// silently dropped (it's not in this chunk because trim_trailing removes the partial
/// tail, and not in the next chunk because align_leading skips past it). For typical
/// log files most lines are well under 1 KiB; 64 KiB is comfortably above the longest
/// realistic structured-log line. Lines longer than this still get dropped, but that's
/// the same failure mode the server already had — this fix doesn't make it worse.
const TRAILING_LOOKAHEAD: u64 = 64 * 1024;

/// Cloneable handle to the per-app state. All handlers receive this via axum's `State`
/// extractor; cloning is just Arc bumps.
#[derive(Clone)]
pub struct AppState {
    pub source: Arc<SourceClient>,
    pub render: RenderConfig,
}

/// Construct the `/api/*` router with the given state attached. The caller is expected
/// to nest this under `/api` and add layers (tracing, compression, …) at the outer
/// level so they cover both the API and the static-asset fallback.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/probe", get(probe))
        .route("/render", get(render))
        .route("/search", get(search))
        .with_state(state)
}

// ---

#[derive(Serialize)]
struct Health {
    ok: bool,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
    })
}

// ---

#[derive(Debug, Deserialize)]
struct ProbeQuery {
    url: String,
}

async fn probe(State(state): State<AppState>, Query(q): Query<ProbeQuery>) -> Result<Json<Metadata>, ApiError> {
    let meta = state.source.probe(&q.url).await?;
    Ok(Json(meta))
}

// ---

#[derive(Debug, Deserialize)]
struct RenderQuery {
    url: String,
    /// Byte offset (inclusive) where the requested range starts in the source file.
    start: u64,
    /// Byte offset (exclusive) where the requested range ends in the source file.
    end: u64,
}

/// Wire format for /api/render. `first_byte` and `last_byte` describe the byte range
/// the server actually rendered, which may differ slightly from what was requested
/// because we trim a leading partial line (when `start > 0`) so the first emitted line
/// is always a full line.
#[derive(Serialize)]
struct RenderResponse {
    first_byte: u64,
    last_byte: u64,
    lines: Vec<RenderedLine>,
}

#[derive(Serialize)]
struct RenderedLine {
    start: u64,
    segments: Vec<Segment>,
}

async fn render(State(state): State<AppState>, Query(q): Query<RenderQuery>) -> Result<Json<RenderResponse>, ApiError> {
    if q.end <= q.start {
        return Ok(Json(RenderResponse {
            first_byte: q.start,
            last_byte: q.start,
            lines: Vec::new(),
        }));
    }
    let span = q.end - q.start;
    if span > MAX_RENDER_BYTES {
        return Err(ApiError::range_too_large(span, MAX_RENDER_BYTES));
    }

    // Fetch one extra byte to the left when q.start > 0 so we can tell whether
    // q.start sits on a line boundary or mid-line. Without this peek the leading
    // alignment would eat the first line whenever the client made a request whose
    // start happened to coincide with the previous chunk's last_byte.
    let (fetch_start, lookahead) = if q.start > 0 { (q.start - 1, true) } else { (0, false) };
    // Fetch slightly past q.end so we can include the full body of the last line
    // whose start sits in [q.start, q.end) but whose newline lands past q.end. The
    // source's get_range clamps to the actual content length so reading past EOF
    // is safe.
    let fetch_end = q.end + TRAILING_LOOKAHEAD;
    let bytes = state.source.get_range(&q.url, fetch_start, fetch_end).await?;

    let (aligned, first_byte) = align_leading(&bytes, fetch_start, lookahead);
    // Each chunk owns the lines whose START byte sits in [q.start, q.end). We
    // include their full content (terminating newlines may be past q.end). The
    // resulting last_byte may exceed q.end, which is fine — the client keys
    // its chunk cache by the requested start, not by where the data lands.
    let aligned = trim_to_chunk_end(aligned, first_byte, q.end);
    let last_byte = first_byte + aligned.len() as u64;

    let mut renderer = state.render.renderer();
    let mut lines = Vec::new();
    renderer.render_chunk(aligned, first_byte, hl::Filter::default(), |r| {
        lines.push(RenderedLine {
            start: r.start,
            segments: ansi_to_segments(r.ansi),
        });
    });

    Ok(Json(RenderResponse {
        first_byte,
        last_byte,
        lines,
    }))
}

/// Align the leading edge of the fetched bytes to a line boundary. With
/// `lookahead = true`, `bytes[0]` is byte `fetch_start` in the source and we use it
/// to decide whether `fetch_start + 1` (= the request's actual start) is on a line
/// boundary — if so, no skip; otherwise skip past the next newline.
fn align_leading(bytes: &[u8], fetch_start: u64, lookahead: bool) -> (&[u8], u64) {
    if bytes.is_empty() {
        return (bytes, fetch_start + if lookahead { 1 } else { 0 });
    }
    if !lookahead {
        // No lookahead means we started at byte 0 of the file, which is by
        // definition the start of line 0.
        return (bytes, fetch_start);
    }
    if bytes[0] == b'\n' {
        // The byte just before the requested start is a newline — so the requested
        // start (= fetch_start + 1) is line-aligned. Skip the lookahead byte only.
        return (&bytes[1..], fetch_start + 1);
    }
    // `bytes[0]` is mid-line. Walk forward to the next newline; the byte after that
    // is the start of the next whole line.
    match bytes.iter().position(|&b| b == b'\n') {
        Some(pos) => (&bytes[pos + 1..], fetch_start + pos as u64 + 1),
        // No newline anywhere — the whole fetched span is one (very long) line.
        // Render nothing; caller can widen and retry.
        None => (&[], fetch_start + bytes.len() as u64),
    }
}

/// Return the prefix of `aligned` that covers exactly the lines whose start byte
/// (in source coordinates) lies in `[first_byte, q_end)`. Each such line is included
/// in full, up to and including its terminating newline — even if that newline lies
/// past `q_end` (caller must have fetched a few extra bytes beyond `q_end` to make
/// this possible).
///
/// Walks line by line: every newline at position `p` terminates the line starting at
/// `cursor`; if that line's start was past `q_end` we stop, otherwise we extend the
/// kept range to include the newline and step `cursor` forward. If a line's body
/// extends past the fetched buffer (no newline found), we drop it — the caller's
/// `TRAILING_LOOKAHEAD` already covers any realistic log line length.
fn trim_to_chunk_end<'a>(aligned: &'a [u8], first_byte: u64, q_end: u64) -> &'a [u8] {
    let mut trimmed_end = 0usize;
    let mut search_from = 0usize;
    loop {
        let line_start = first_byte + search_from as u64;
        if line_start >= q_end {
            break;
        }
        match aligned[search_from..].iter().position(|&b| b == b'\n') {
            Some(p) => {
                trimmed_end = search_from + p + 1;
                search_from = trimmed_end;
            }
            None => break,
        }
    }
    &aligned[..trimmed_end]
}

// ---

#[derive(Debug, Deserialize)]
struct SearchQuery {
    url: String,
    q: String,
    #[serde(default)]
    mode: SearchMode,
    start: u64,
    end: u64,
    /// Case-insensitive substring matching. Defaults to true to match what
    /// browser-native find behaves like. Ignored in query mode.
    #[serde(default = "default_case_insensitive")]
    case_insensitive: bool,
}

fn default_case_insensitive() -> bool {
    true
}

#[derive(Serialize)]
struct SearchResponse {
    first_byte: u64,
    last_byte: u64,
    matches: Vec<SearchMatch>,
}

async fn search(State(state): State<AppState>, Query(q): Query<SearchQuery>) -> Result<Json<SearchResponse>, ApiError> {
    if q.end <= q.start {
        return Ok(Json(SearchResponse {
            first_byte: q.start,
            last_byte: q.start,
            matches: Vec::new(),
        }));
    }
    let span = q.end - q.start;
    if span > MAX_RENDER_BYTES {
        return Err(ApiError::range_too_large(span, MAX_RENDER_BYTES));
    }

    let (fetch_start, lookahead) = if q.start > 0 { (q.start - 1, true) } else { (0, false) };
    let fetch_end = q.end + TRAILING_LOOKAHEAD;
    let bytes = state.source.get_range(&q.url, fetch_start, fetch_end).await?;
    let (aligned, first_byte) = align_leading(&bytes, fetch_start, lookahead);
    let aligned = trim_to_chunk_end(aligned, first_byte, q.end);
    let last_byte = first_byte + aligned.len() as u64;

    let matches = search_chunk(&state.render, aligned, first_byte, &q.q, q.mode, q.case_insensitive)?;

    Ok(Json(SearchResponse {
        first_byte,
        last_byte,
        matches,
    }))
}

// ---

/// Wire-format error body. Stable shape so the client can branch on `kind` without
/// reading the human message.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
    kind: &'static str,
}

pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
}

impl ApiError {
    fn range_too_large(span: u64, max: u64) -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            body: ErrorBody {
                error: format!("requested {span} bytes exceeds per-request limit {max}"),
                kind: "range_too_large",
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<SearchError> for ApiError {
    fn from(e: SearchError) -> Self {
        let (status, kind) = match &e {
            SearchError::InvalidQuery(_) => (StatusCode::BAD_REQUEST, "invalid_query"),
        };
        ApiError {
            status,
            body: ErrorBody {
                error: e.to_string(),
                kind,
            },
        }
    }
}

impl From<SourceError> for ApiError {
    fn from(e: SourceError) -> Self {
        let (status, kind) = match &e {
            SourceError::InvalidUrl(_) => (StatusCode::BAD_REQUEST, "invalid_url"),
            SourceError::SchemeNotAllowed(_) => (StatusCode::BAD_REQUEST, "scheme_not_allowed"),
            SourceError::HostNotAllowed(_) => (StatusCode::FORBIDDEN, "host_not_allowed"),
            SourceError::AddressBlocked { .. } => (StatusCode::FORBIDDEN, "address_blocked"),
            SourceError::DnsFailed { .. } => (StatusCode::BAD_GATEWAY, "dns_failed"),
            SourceError::UpstreamStatus { status } => (
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY),
                "upstream_status",
            ),
            SourceError::TooLarge { .. } => (StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
            SourceError::Http(_) => (StatusCode::BAD_GATEWAY, "upstream_http"),
            SourceError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "io_error"),
        };
        if status.is_server_error() {
            warn!(?e, "source error surfaced as {}", status);
        }
        ApiError {
            status,
            body: ErrorBody {
                error: e.to_string(),
                kind,
            },
        }
    }
}
