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

use crate::source::{Metadata, SourceClient, SourceError};

/// Cloneable handle to the per-app state. All handlers receive this via axum's `State`
/// extractor; cloning is just an Arc bump.
#[derive(Clone)]
pub struct AppState {
    pub source: Arc<SourceClient>,
}

/// Construct the `/api/*` router with the given state attached. The caller is expected
/// to nest this under `/api` and add layers (tracing, compression, …) at the outer
/// level so they cover both the API and the static-asset fallback.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/probe", get(probe))
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

async fn probe(
    State(state): State<AppState>,
    Query(q): Query<ProbeQuery>,
) -> Result<Json<Metadata>, ApiError> {
    let meta = state.source.probe(&q.url).await?;
    Ok(Json(meta))
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
