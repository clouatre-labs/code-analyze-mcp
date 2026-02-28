use code_analyze_mcp::CodeAnalyzer;
use rmcp::serve_server;
use rmcp::transport::stdio;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let analyzer = CodeAnalyzer::new();
    let (stdin, stdout) = stdio();

    serve_server(analyzer, (stdin, stdout)).await?;

    Ok(())
}
