// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
use code_analyze_mcp::logging::{LogEvent, level_to_mcp};
use rmcp::model::{CallToolResult, Content, LoggingLevel, Meta};
#[test]
fn test_logging_level_to_mcp_mapping() {
    use tracing::Level;

    assert_eq!(level_to_mcp(&Level::TRACE), LoggingLevel::Debug);
    assert_eq!(level_to_mcp(&Level::DEBUG), LoggingLevel::Debug);
    assert_eq!(level_to_mcp(&Level::INFO), LoggingLevel::Info);
    assert_eq!(level_to_mcp(&Level::WARN), LoggingLevel::Warning);
    assert_eq!(level_to_mcp(&Level::ERROR), LoggingLevel::Error);
}

#[tokio::test]
async fn test_batch_draining_with_multiple_events() {
    use serde_json::json;

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<LogEvent>();

    for i in 0..5 {
        let log_event = LogEvent {
            level: LoggingLevel::Info,
            logger: format!("logger_{i}"),
            data: json!({"index": i}),
        };
        let _ = event_tx.send(log_event);
    }

    let mut buffer = Vec::with_capacity(64);
    event_rx.recv_many(&mut buffer, 64).await;

    assert_eq!(buffer.len(), 5);
    for (i, event) in buffer.iter().enumerate() {
        assert_eq!(event.logger, format!("logger_{i}"));
        assert_eq!(event.data, json!({"index": i}));
    }
}

#[test]
fn test_call_tool_result_cache_hint_metadata() {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "cache_hint".to_string(),
        serde_json::Value::String("no-cache".to_string()),
    );

    let result =
        CallToolResult::success(vec![Content::text("test output")]).with_meta(Some(Meta(meta)));

    let json_val = serde_json::to_value(&result).expect("should serialize");

    assert_eq!(
        json_val
            .get("_meta")
            .and_then(|m| m.get("cache_hint"))
            .and_then(|v| v.as_str()),
        Some("no-cache"),
        "Expected _meta.cache_hint to be 'no-cache' in serialized JSON: {json_val}"
    );
}
