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
use crate::segments::{Segment, ansi_to_segments};
use crate::source::{Metadata, SourceClient, SourceError};

/// Cap on the byte range any single /api/render request may ask for. The client is
/// expected to paginate; this is a defence against an accidental "render the whole 2
/// GiB log in one call." Tunable later if it becomes the bottleneck.
const MAX_RENDER_BYTES: u64 = 16 * 1024 * 1024;

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

    let bytes = state.source.get_range(&q.url, q.start, q.end).await?;

    // If we requested a non-zero start, the first bytes likely belong to the tail of a
    // line we don't fully see. Skip to the first newline so every rendered line is a
    // complete line in the source.
    let (aligned, first_byte) = align_leading(&bytes, q.start, q.start > 0);

    let mut renderer = state.render.make_renderer();
    let mut lines = Vec::new();
    renderer.render_chunk(aligned, first_byte, |r| {
        lines.push(RenderedLine {
            start: r.start,
            segments: ansi_to_segments(r.ansi),
        });
    });
    let last_byte = first_byte + aligned.len() as u64;

    Ok(Json(RenderResponse {
        first_byte,
        last_byte,
        lines,
    }))
}

fn align_leading(bytes: &[u8], start: u64, skip_partial: bool) -> (&[u8], u64) {
    if !skip_partial || bytes.is_empty() {
        return (bytes, start);
    }
    match bytes.iter().position(|&b| b == b'\n') {
        Some(pos) => (&bytes[pos + 1..], start + pos as u64 + 1),
        // No newline in the slice — the whole thing is a fragment of one line. Render
        // nothing; the caller can widen the range and retry.
        None => (&[], start + bytes.len() as u64),
    }
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
