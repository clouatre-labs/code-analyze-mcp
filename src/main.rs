use code_analyze_mcp::{CodeAnalyzer, logging::McpLoggingLayer};
use rmcp::serve_server;
use rmcp::transport::stdio;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create shared peer Arc for logging layer
    let peer = Arc::new(TokioMutex::new(None));

    // Create shared level filter for dynamic control (std::sync::Mutex for Copy type)
    let log_level_filter = Arc::new(Mutex::new(LevelFilter::WARN));

    // Create MCP logging layer with filter
    let mcp_logging_layer = McpLoggingLayer::new(peer.clone(), log_level_filter.clone());

    // Build layered subscriber: fmt + MCP logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(mcp_logging_layer)
        .init();

    let analyzer = CodeAnalyzer::new(peer, log_level_filter);
    let (stdin, stdout) = stdio();

    serve_server(analyzer, (stdin, stdout)).await?;

    Ok(())
}
