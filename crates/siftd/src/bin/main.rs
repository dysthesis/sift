use std::{io::IsTerminal as _, path::Path};

use axum::{routing::post, Router};
use clap::Parser;
use color_eyre::{eyre::eyre, Result};
use libsift::handler::url::handle_url;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::cli::{Cli, ColorChoice, LogFormat, LogLevel};

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Install color_eyre first for panic and error reports
    color_eyre::install()?;

    let cli = Cli::parse();

    // Initialize log compatibility for `log`-using deps
    LogTracer::init().ok();
    init_tracing(&cli)?;

    let header = axum::http::HeaderName::from_static("x-request-id");
    let include_queries = cli.log_queries;
    let trace_layer = {
        use axum::http::Request;
        use std::time::Duration;
        use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
        use tracing::{field, Span};

        TraceLayer::new(SharedClassifier::new(ServerErrorsAsFailures::new()))
            .make_span_with(move |req: &Request<_>| {
                let method = req.method();
                let path = req.uri().path();
                let query = req.uri().query();
                let client_ip = req
                    .headers()
                    .get("x-forwarded-for")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                let request_id = req
                    .headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                let span = tracing::span!(
                    Level::INFO,
                    "http",
                    %method,
                    %path,
                    request_id = %request_id,
                    client_ip = %client_ip,
                    url_qs = field::Empty,
                );
                if include_queries {
                    span.record("url_qs", field::display(query.unwrap_or("")));
                }
                span
            })
            .on_failure(|error, latency: Duration, _span: &Span| {
                tracing::error!(err = %error, elapsed_ms = latency.as_millis() as u64, "request failure");
            })
    };

    let app = Router::new()
        .route("/url", post(handle_url))
        .layer(PropagateRequestIdLayer::new(header.clone()))
        .layer(SetRequestIdLayer::new(header, MakeRequestUuid))
        .layer(trace_layer);

    let bind_addr = format!("localhost:{}", cli.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!(addr = %bind_addr, version = env!("CARGO_PKG_VERSION"), "server start");
    let result = axum::serve(listener, app).await.map_err(|e| eyre!(e));
    info!("server shutdown");
    result
}

fn init_tracing(cli: &Cli) -> Result<()> {
    // Determine env filter precedence: CLI filter > SIFT_LOG/RUST_LOG > constructed defaults
    let env_filter = if let Some(f) = &cli.log_filter {
        EnvFilter::new(f)
    } else if let Ok(s) = std::env::var("SIFT_LOG").or_else(|_| std::env::var("RUST_LOG")) {
        EnvFilter::new(s)
    } else {
        // Compose base level for siftd/libsift from CLI level/verbosity, others warn by default
        let base = match (cli.quiet, cli.log_level, cli.verbose) {
            (true, _, _) => "warn",
            (false, Some(LogLevel::Error), _) => "error",
            (false, Some(LogLevel::Warn), _) => "warn",
            (false, Some(LogLevel::Info), _) => "info",
            (false, Some(LogLevel::Debug), _) => "debug",
            (false, Some(LogLevel::Trace), _) => "trace",
            // -v mapping
            (false, None, v) if v >= 3 => "trace",
            (false, None, 2) => "debug",
            (false, None, 1) => "info",
            _ => "info",
        };
        let s = format!(
            "siftd={base},libsift={base},tower_http=info,axum=info,hyper=warn,reqwest=warn,sqlx=warn"
        );
        EnvFilter::new(s)
    };

    // Choose writer: file (non-blocking) or stderr
    // Color handling
    let ansi = match cli.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            if std::env::var_os("NO_COLOR").is_some() {
                false
            } else {
                // Use same stream that writer targets; if writer is a file, disable color.
                // When to stderr, detect TTY.
                cli.log_file.is_none() && std::io::stderr().is_terminal()
            }
        }
    };

    // Determine format
    let use_json = match cli.log_format {
        LogFormat::Json => true,
        LogFormat::Pretty | LogFormat::Compact => false,
        LogFormat::Auto => {
            // If writing to file or not a TTY, default to JSON
            if cli.log_file.is_some() {
                true
            } else {
                !std::io::stderr().is_terminal()
            }
        }
    };
    static LOG_GUARD: once_cell::sync::OnceCell<tracing_appender::non_blocking::WorkerGuard> =
        once_cell::sync::OnceCell::new();
    if use_json {
        if let Some(ref path) = cli.log_file {
            let (non_blocking, guard) = non_blocking_file(path);
            let _ = LOG_GUARD.set(guard);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .event_format(fmt::format().json().flatten_event(true))
                        .with_ansi(false)
                        .with_writer(non_blocking),
                )
                .with(ErrorLayer::default())
                .try_init()
                .ok();
        } else {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .event_format(fmt::format().json().flatten_event(true))
                        .with_ansi(false)
                        .with_writer(std::io::stderr),
                )
                .with(ErrorLayer::default())
                .try_init()
                .ok();
        }
    } else {
        match (cli.log_format, &cli.log_file) {
            (LogFormat::Pretty | LogFormat::Auto, Some(path)) => {
                let (non_blocking, guard) = non_blocking_file(path);
                let _ = LOG_GUARD.set(guard);
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        fmt::layer()
                            .pretty()
                            .with_ansi(ansi)
                            .with_writer(non_blocking),
                    )
                    .with(ErrorLayer::default())
                    .try_init()
                    .ok();
            }
            (LogFormat::Compact, Some(path)) => {
                let (non_blocking, guard) = non_blocking_file(path);
                let _ = LOG_GUARD.set(guard);
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        fmt::layer()
                            .compact()
                            .with_ansi(ansi)
                            .with_writer(non_blocking),
                    )
                    .with(ErrorLayer::default())
                    .try_init()
                    .ok();
            }
            (LogFormat::Pretty | LogFormat::Auto, None) => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        fmt::layer()
                            .pretty()
                            .with_ansi(ansi)
                            .with_writer(std::io::stderr),
                    )
                    .with(ErrorLayer::default())
                    .try_init()
                    .ok();
            }
            (LogFormat::Compact, None) => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        fmt::layer()
                            .compact()
                            .with_ansi(ansi)
                            .with_writer(std::io::stderr),
                    )
                    .with(ErrorLayer::default())
                    .try_init()
                    .ok();
            }
            (LogFormat::Json, _) => unreachable!(),
        }
    }
    Ok(())
}

fn non_blocking_file(
    path: &Path,
) -> (
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
) {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("failed to open log file");
    tracing_appender::non_blocking(file)
}

#[allow(dead_code)]
fn strip_query_fragment(url: &str) -> &str {
    // Helper for redaction if needed elsewhere
    url.split(['?', '#']).next().unwrap_or(url)
}
