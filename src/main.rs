use axum::{
    Router,
    body::Body,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
};
use code_analyze_mcp::{CodeAnalyzer, logging::McpLoggingLayer};
use rmcp::serve_server;
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Clone, Copy)]
enum Transport {
    Stdio,
    Http,
}

#[derive(Debug)]
struct CliArgs {
    transport: Transport,
    port: u16,
}

fn parse_args() -> Result<CliArgs, String> {
    let mut transport = Transport::Stdio;
    let mut port = 8080u16;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" => {
                println!("Usage: code-analyze-mcp [OPTIONS]");
                println!("Options:");
                println!("  --transport <stdio|http>  Transport type (default: stdio)");
                println!("  --port <PORT>             Port for HTTP transport (default: 8080)");
                println!("  --help                    Show this help message");
                std::process::exit(0);
            }
            "--transport" => {
                let value = args.next().ok_or("--transport requires a value")?;
                transport = match value.as_str() {
                    "stdio" => Transport::Stdio,
                    "http" => Transport::Http,
                    _ => return Err(format!("Invalid transport: {}", value)),
                };
            }
            "--port" => {
                let value = args.next().ok_or("--port requires a value")?;
                port = value
                    .parse()
                    .map_err(|_| format!("Invalid port: {}", value))?;
            }
            _ => return Err(format!("Unknown argument: {}", arg)),
        }
    }

    Ok(CliArgs { transport, port })
}

async fn origin_validation_middleware(
    req: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(origin) = req.headers().get("origin") {
        let origin_str = origin.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;

        let is_localhost = origin_str == "http://127.0.0.1"
            || origin_str == "http://localhost"
            || origin_str == "http://[::1]"
            || origin_str.starts_with("http://127.0.0.1:")
            || origin_str.starts_with("http://localhost:")
            || origin_str.starts_with("http://[::1]:");

        if !is_localhost {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(next.run(req).await)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    })?;

    // Create shared peer Arc for logging layer
    let peer = Arc::new(TokioMutex::new(None));

    // Create shared level filter for dynamic control (std::sync::Mutex for Copy type)
    let log_level_filter = Arc::new(Mutex::new(LevelFilter::WARN));

    // Create unbounded channel for log events
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create MCP logging layer with event sender
    let mcp_logging_layer = McpLoggingLayer::new(event_tx, log_level_filter.clone());

    // Build layered subscriber: fmt + MCP logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(mcp_logging_layer)
        .init();

    match args.transport {
        Transport::Stdio => {
            info!("Starting MCP server with stdio transport");
            let analyzer = CodeAnalyzer::new(peer, log_level_filter, event_rx);
            let (stdin, stdout) = stdio();

            let service = serve_server(analyzer, (stdin, stdout)).await?;
            service.waiting().await?;
        }
        Transport::Http => {
            info!(
                "Starting MCP server with HTTP transport on 127.0.0.1:{}",
                args.port
            );

            let ct = CancellationToken::new();
            let ct_clone = ct.clone();

            let config = StreamableHttpServerConfig {
                stateful_mode: true,
                cancellation_token: ct.child_token(),
                ..Default::default()
            };

            let session_manager = Arc::new(LocalSessionManager::default());

            let service: StreamableHttpService<CodeAnalyzer, LocalSessionManager> =
                StreamableHttpService::new(
                    move || {
                        let peer = Arc::new(TokioMutex::new(None));
                        let log_level_filter = Arc::new(Mutex::new(LevelFilter::WARN));
                        let (_event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
                        Ok(CodeAnalyzer::new(peer, log_level_filter, event_rx))
                    },
                    session_manager,
                    config,
                );

            let router = Router::new()
                .nest_service("/mcp", service)
                .layer(middleware::from_fn(origin_validation_middleware));

            let bind_addr = format!("127.0.0.1:{}", args.port);
            let tcp_listener = tokio::net::TcpListener::bind(&bind_addr).await?;
            info!("HTTP server listening on {}", bind_addr);

            let server = axum::serve(tcp_listener, router).with_graceful_shutdown(async move {
                ct_clone.cancelled().await;
            });

            tokio::spawn(async move {
                if let Err(e) = tokio::signal::ctrl_c().await {
                    error!("Failed to listen for ctrl-c: {}", e);
                } else {
                    info!("Received ctrl-c, shutting down gracefully");
                    ct.cancel();
                }
            });

            server.await?;
        }
    }

    Ok(())
}
