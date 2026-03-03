use rmcp::Peer;
use rmcp::RoleServer;
use rmcp::model::{
    LoggingLevel, LoggingMessageNotificationParam, Notification, ServerNotification,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use tracing::span::Attributes;
use tracing::subscriber::Interest;
use tracing::{Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::Context;

/// Maps tracing::Level to MCP LoggingLevel.
pub fn level_to_mcp(level: &Level) -> LoggingLevel {
    match *level {
        Level::TRACE | Level::DEBUG => LoggingLevel::Debug,
        Level::INFO => LoggingLevel::Info,
        Level::WARN => LoggingLevel::Warning,
        Level::ERROR => LoggingLevel::Error,
    }
}

/// Custom tracing Layer that bridges tracing events to MCP client via peer.notify_logging_message().
/// Holds a shared reference to the peer (set via on_initialized) and the log level filter.
pub struct McpLoggingLayer {
    peer: Arc<TokioMutex<Option<Peer<RoleServer>>>>,
    log_level_filter: Arc<Mutex<LevelFilter>>,
}

impl McpLoggingLayer {
    pub fn new(
        peer: Arc<TokioMutex<Option<Peer<RoleServer>>>>,
        log_level_filter: Arc<Mutex<LevelFilter>>,
    ) -> Self {
        Self {
            peer,
            log_level_filter,
        }
    }
}

impl<S> Layer<S> for McpLoggingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = *metadata.level();
        let target = metadata.target();

        // Check if event level passes the current filter before processing
        let filter_level = self.log_level_filter.lock().unwrap();
        if level > *filter_level {
            return;
        }
        drop(filter_level);

        // Extract message from the event using a visitor.
        let mut message = String::new();
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);

        let mcp_level = level_to_mcp(&level);

        // Spawn async task to send notification without blocking on_event.
        let peer = self.peer.clone();
        let logger = target.to_string();
        let msg = message.clone();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let peer_lock = peer.lock().await;
                if let Some(peer) = peer_lock.as_ref() {
                    let notification = ServerNotification::LoggingMessageNotification(
                        Notification::new(LoggingMessageNotificationParam {
                            level: mcp_level,
                            logger: Some(logger),
                            data: json!(msg),
                        }),
                    );
                    if let Err(e) = peer.send_notification(notification).await {
                        tracing::warn!("Failed to send logging notification: {}", e);
                    }
                }
            });
        }
    }

    fn register_callsite(&self, metadata: &'static tracing::Metadata<'static>) -> Interest {
        let filter_level = self.log_level_filter.lock().unwrap();
        if *metadata.level() <= *filter_level {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        let filter_level = self.log_level_filter.lock().unwrap();
        *metadata.level() <= *filter_level
    }

    fn on_new_span(&self, _attrs: &Attributes<'_>, _id: &tracing::span::Id, _ctx: Context<'_, S>) {}
}

/// Visitor to extract message from tracing event.
struct MessageVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0.push_str(&format!("{:?}", value));
        } else {
            self.0.push_str(&format!("{}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            self.0.push_str(&format!("{}={}", field.name(), value));
        }
    }
}
