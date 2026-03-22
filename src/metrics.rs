//! Metrics collection and daily-rotating JSONL emission.
//!
//! Provides a channel-based pipeline: callers emit [`MetricEvent`] values via [`MetricsSender`],
//! and [`MetricsWriter`] drains the channel and appends events to a daily-rotated JSONL file
//! under the XDG data directory (`~/.local/share/code-analyze-mcp/metrics-YYYY-MM-DD.jsonl`).
//! Files older than 30 days are deleted on startup.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

/// A single metric event emitted by a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEvent {
    pub ts: u64,
    pub tool: &'static str,
    pub duration_ms: u64,
    pub output_chars: usize,
    pub param_path_depth: usize,
    pub max_depth: Option<u32>,
    pub result: &'static str,
    pub error_type: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub seq: Option<u32>,
}

/// Sender half of the metrics channel; cloned and passed to tools for event emission.
#[derive(Clone)]
pub struct MetricsSender(pub tokio::sync::mpsc::UnboundedSender<MetricEvent>);

impl MetricsSender {
    pub fn send(&self, event: MetricEvent) {
        let _ = self.0.send(event);
    }
}

/// Receiver half of the metrics channel; drains events and writes them to daily-rotated JSONL files.
pub struct MetricsWriter {
    rx: tokio::sync::mpsc::UnboundedReceiver<MetricEvent>,
    base_dir: PathBuf,
}

impl MetricsWriter {
    pub fn new(
        rx: tokio::sync::mpsc::UnboundedReceiver<MetricEvent>,
        base_dir: Option<PathBuf>,
    ) -> Self {
        let dir = base_dir.unwrap_or_else(xdg_metrics_dir);
        cleanup_old_files(&dir);
        Self { rx, base_dir: dir }
    }

    pub async fn run(mut self) {
        let mut current_date = current_date_str();
        let mut current_file: Option<PathBuf> = None;

        loop {
            let mut batch = Vec::new();
            if let Some(event) = self.rx.recv().await {
                batch.push(event);
                for _ in 0..99 {
                    match self.rx.try_recv() {
                        Ok(e) => batch.push(e),
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => break,
                    }
                }
            } else {
                break;
            }

            let new_date = current_date_str();
            if new_date != current_date {
                current_date = new_date;
                current_file = None;
            }

            if current_file.is_none() {
                current_file = Some(rotate_path(&self.base_dir, &current_date));
            }

            let path = current_file.as_ref().unwrap();

            // Create directory once per batch
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                tokio::fs::create_dir_all(parent).await.ok();
            }

            // Open file once per batch
            let file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .await;

            if let Ok(mut file) = file {
                for event in batch {
                    if let Ok(json) = serde_json::to_string(&event) {
                        let _ = file.write_all(json.as_bytes()).await;
                        let _ = file.write_all(b"\n").await;
                    }
                }
            }
        }
    }
}

/// Returns the current UNIX timestamp in milliseconds.
pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Counts the number of path segments in a file path.
pub fn path_component_count(path: &str) -> usize {
    Path::new(path).components().count()
}

/// Maps an MCP error code to a short string representation for metrics.
pub fn error_code_to_type(code: rmcp::model::ErrorCode) -> &'static str {
    match code {
        rmcp::model::ErrorCode::PARSE_ERROR => "parse",
        rmcp::model::ErrorCode::INVALID_PARAMS => "invalid_params",
        rmcp::model::ErrorCode::METHOD_NOT_FOUND => "unknown",
        rmcp::model::ErrorCode::INTERNAL_ERROR => "unknown",
        _ => "unknown",
    }
}

fn xdg_metrics_dir() -> PathBuf {
    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME")
        && !xdg_data_home.is_empty()
    {
        return PathBuf::from(xdg_data_home).join("code-analyze-mcp");
    }

    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("code-analyze-mcp")
    } else {
        PathBuf::from(".")
    }
}

fn rotate_path(base_dir: &Path, date_str: &str) -> PathBuf {
    base_dir.join(format!("metrics-{}.jsonl", date_str))
}

fn cleanup_old_files(base_dir: &Path) {
    let now_days = (unix_ms() / 86_400_000) as u32;

    let Ok(entries) = std::fs::read_dir(base_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = match path.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => continue,
        };

        // Expected format: metrics-YYYY-MM-DD.jsonl
        if !file_name.starts_with("metrics-") || !file_name.ends_with(".jsonl") {
            continue;
        }
        let date_part = &file_name[8..file_name.len() - 6];
        if date_part.len() != 10
            || date_part.as_bytes().get(4) != Some(&b'-')
            || date_part.as_bytes().get(7) != Some(&b'-')
        {
            continue;
        }
        let Ok(year) = date_part[0..4].parse::<u32>() else {
            continue;
        };
        let Ok(month) = date_part[5..7].parse::<u32>() else {
            continue;
        };
        let Ok(day) = date_part[8..10].parse::<u32>() else {
            continue;
        };
        if month == 0 || month > 12 || day == 0 || day > 31 {
            continue;
        }

        let file_days = date_to_days_since_epoch(year, month, day);
        if now_days > file_days && (now_days - file_days) > 30 {
            let _ = std::fs::remove_file(&path);
        }
    }
}

fn date_to_days_since_epoch(y: u32, m: u32, d: u32) -> u32 {
    // Shift year so March is month 0
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097
        + doe // this is the Proleptic Gregorian day number
            .saturating_sub(719_468) // subtract the epoch offset to get days since 1970-01-01
}

/// Returns the current UTC date as a string in YYYY-MM-DD format.
pub fn current_date_str() -> String {
    let days = (unix_ms() / 86_400_000) as u32;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_event_serialization() {
        let event = MetricEvent {
            ts: 1_700_000_000_000,
            tool: "analyze_directory",
            duration_ms: 42,
            output_chars: 100,
            param_path_depth: 3,
            max_depth: Some(2),
            result: "ok",
            error_type: None,
            session_id: None,
            seq: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("analyze_directory"));
        assert!(json.contains(r#""result":"ok""#));
        assert!(json.contains(r#""output_chars":100"#));
    }

    #[test]
    fn test_metric_event_serialization_error() {
        let event = MetricEvent {
            ts: 1_700_000_000_000,
            tool: "analyze_directory",
            duration_ms: 5,
            output_chars: 0,
            param_path_depth: 3,
            max_depth: Some(3),
            result: "error",
            error_type: Some("invalid_params".to_string()),
            session_id: None,
            seq: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""result":"error""#));
        assert!(json.contains(r#""error_type":"invalid_params""#));
        assert!(json.contains(r#""output_chars":0"#));
        assert!(json.contains(r#""max_depth":3"#));
    }

    #[test]
    fn test_metric_event_new_fields_round_trip() {
        let event = MetricEvent {
            ts: 1_700_000_000_000,
            tool: "analyze_file",
            duration_ms: 100,
            output_chars: 500,
            param_path_depth: 2,
            max_depth: Some(3),
            result: "ok",
            error_type: None,
            session_id: Some("1742468880123-42".to_string()),
            seq: Some(5),
        };
        let serialized = serde_json::to_string(&event).unwrap();
        let json_str = r#"{"ts":1700000000000,"tool":"analyze_file","duration_ms":100,"output_chars":500,"param_path_depth":2,"max_depth":3,"result":"ok","error_type":null,"session_id":"1742468880123-42","seq":5}"#;
        assert_eq!(serialized, json_str);
        let parsed: MetricEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed.session_id, Some("1742468880123-42".to_string()));
        assert_eq!(parsed.seq, Some(5));
    }

    #[test]
    fn test_metric_event_backward_compat_parse() {
        let old_jsonl = r#"{"ts":1700000000000,"tool":"analyze_directory","duration_ms":42,"output_chars":100,"param_path_depth":3,"max_depth":2,"result":"ok","error_type":null}"#;
        let parsed: MetricEvent = serde_json::from_str(old_jsonl).unwrap();
        assert_eq!(parsed.tool, "analyze_directory");
        assert_eq!(parsed.session_id, None);
        assert_eq!(parsed.seq, None);
    }

    #[test]
    fn test_session_id_format() {
        let event = MetricEvent {
            ts: 1_700_000_000_000,
            tool: "analyze_symbol",
            duration_ms: 20,
            output_chars: 50,
            param_path_depth: 1,
            max_depth: None,
            result: "ok",
            error_type: None,
            session_id: Some("1742468880123-0".to_string()),
            seq: Some(0),
        };
        let sid = event.session_id.unwrap();
        assert!(sid.contains('-'), "session_id should contain a dash");
        let parts: Vec<&str> = sid.split('-').collect();
        assert_eq!(parts.len(), 2, "session_id should have exactly 2 parts");
        assert!(parts[0].len() == 13, "millis part should be 13 digits");
    }
}
