use axum::{
    Router,
    body::Body,
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
};
use code_analyze_mcp::CodeAnalyzer;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::filter::LevelFilter;

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

#[tokio::test]
async fn test_http_transport_initialize() {
    let ct = CancellationToken::new();
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

    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to localhost");
    let addr = tcp_listener
        .local_addr()
        .expect("Failed to get local address");

    let ct_clone = ct.clone();
    let server_handle = tokio::spawn(async move {
        let server = axum::serve(tcp_listener, router).with_graceful_shutdown(async move {
            ct_clone.cancelled().await;
        });
        let _ = server.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    let initialize_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&initialize_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);

    let body_text = response.text().await.expect("Failed to read response body");

    assert!(!body_text.is_empty());
    assert!(body_text.contains("jsonrpc") || body_text.contains("data:"));

    ct.cancel();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), server_handle).await;
}

#[tokio::test]
async fn test_http_transport_rejects_invalid_origin() {
    let ct = CancellationToken::new();
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

    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to localhost");
    let addr = tcp_listener
        .local_addr()
        .expect("Failed to get local address");

    let ct_clone = ct.clone();
    let server_handle = tokio::spawn(async move {
        let server = axum::serve(tcp_listener, router).with_graceful_shutdown(async move {
            ct_clone.cancelled().await;
        });
        let _ = server.await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let url = format!("http://{}/mcp", addr);

    let initialize_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Origin", "http://evil.example.com")
        .json(&initialize_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    ct.cancel();
    let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), server_handle).await;
}
