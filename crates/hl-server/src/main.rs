//! hl-server: native HTTP backend that renders log files via the hl pipeline.
//!
//! The viewer in `www/` is a thin client: it asks this server for byte ranges of a
//! log URL, gets back styled records as JSON, and paints them. All parsing and
//! formatting work happens here, in native Rust, with the hl pipeline's parallelism
//! intact — none of it runs in the browser.

mod api;
mod render;
mod segments;
mod source;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::Router;
use clap::Parser;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;

use crate::render::RenderConfig;
use crate::source::{Config as SourceConfig, HostPattern, SourceClient};

/// CLI surface for the binary. Keep it small — anything the user might want to override
/// from a `systemd` unit or container env goes through here.
#[derive(Debug, Parser)]
#[command(
    name = "hl-server",
    version,
    about = "Native log-viewer backend for the hl thin web client."
)]
struct Cli {
    /// Address to bind the HTTP listener on.
    #[arg(long, default_value = "127.0.0.1:8080", env = "HL_SERVER_BIND")]
    bind: SocketAddr,

    /// Directory containing the static viewer assets (HTML, JS, CSS, themes).
    /// Defaults to the crate-local www/ tree, which works for development.
    #[arg(long, default_value = "crates/hl-server/www", env = "HL_SERVER_WWW_DIR")]
    www_dir: PathBuf,

    /// Allow source URLs that resolve to private / loopback / link-local addresses.
    /// Off by default — turn on only for local dev / test rigs.
    #[arg(long, env = "HL_SERVER_ALLOW_PRIVATE")]
    allow_private: bool,

    /// Allow `file://` source URLs (server reads its own filesystem). Off by default.
    /// Testing only — exposes the server host's files to anyone who can hit the API.
    #[arg(long, env = "HL_SERVER_ALLOW_FILE")]
    allow_file_scheme: bool,

    /// Restrict source URLs to hostnames matching one of these glob patterns. Repeat
    /// the flag (or comma-separate) to add more. When omitted, no host restriction is
    /// applied — any DNS name that survives the SSRF guard is allowed.
    #[arg(
        long = "allow-host",
        value_name = "PATTERN",
        env = "HL_SERVER_ALLOW_HOSTS",
        value_delimiter = ','
    )]
    allow_hosts: Vec<String>,

    /// Maximum source size in bytes. Sources whose `Content-Length` exceeds this are
    /// refused at probe time. Default 10 GiB.
    #[arg(
        long,
        default_value_t = 10 * 1024 * 1024 * 1024,
        env = "HL_SERVER_MAX_SIZE"
    )]
    max_size: u64,

    /// TCP connect timeout for source fetches (seconds).
    #[arg(long, default_value_t = 10, env = "HL_SERVER_CONNECT_TIMEOUT_SECS")]
    connect_timeout_secs: u64,

    /// Total request timeout for source fetches (seconds).
    #[arg(long, default_value_t = 120, env = "HL_SERVER_REQUEST_TIMEOUT_SECS")]
    request_timeout_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let app = build_app(&cli)?;
    let listener = tokio::net::TcpListener::bind(cli.bind)
        .await
        .with_context(|| format!("failed to bind {}", cli.bind))?;
    let bound = listener.local_addr().unwrap_or(cli.bind);
    info!(addr = %bound, www_dir = %cli.www_dir.display(), "hl-server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum::serve failed")?;
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,hl_server=info,tower_http=info"));
    fmt().with_env_filter(filter).with_target(false).init();
}

fn build_app(cli: &Cli) -> anyhow::Result<Router> {
    let source = SourceClient::new(source_config(cli))?;
    let render = RenderConfig::new()?;
    let state = api::AppState {
        source: Arc::new(source),
        render,
    };
    let static_dir = ServeDir::new(&cli.www_dir);
    let app = Router::new()
        .nest("/api", api::router(state))
        .fallback_service(static_dir)
        .layer(TraceLayer::new_for_http());
    Ok(app)
}

fn source_config(cli: &Cli) -> SourceConfig {
    SourceConfig {
        allow_private: cli.allow_private,
        allow_file_scheme: cli.allow_file_scheme,
        allow_hosts: cli.allow_hosts.iter().map(HostPattern::new).collect(),
        max_size: cli.max_size,
        connect_timeout: Duration::from_secs(cli.connect_timeout_secs),
        request_timeout: Duration::from_secs(cli.request_timeout_secs),
    }
}

/// Resolve Ctrl-C or SIGTERM into a graceful shutdown signal for axum::serve.
async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl-C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
