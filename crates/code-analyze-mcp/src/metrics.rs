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
        Self { rx, base_dir: dir }
    }

    pub async fn run(mut self) {
        cleanup_old_files(&self.base_dir).await;
        let mut current_date = current_date_str();
        let mut current_file: Option<PathBuf> = None;

        loop {
            let mut batch = Vec::new();
            if let Some(event) = self.rx.recv().await {
                batch.push(event);
                for _ in 0..99 {
                    match self.rx.try_recv() {
                        Ok(e) => batch.push(e),
                        Err(
                            mpsc::error::TryRecvError::Empty
                            | mpsc::error::TryRecvError::Disconnected,
                        ) => break,
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
                    if let Ok(mut json) = serde_json::to_string(&event) {
                        json.push('\n');
                        let _ = file.write_all(json.as_bytes()).await;
                    }
                }
                let _ = file.flush().await;
            }
        }
    }
}

/// Returns the current UNIX timestamp in milliseconds.
#[must_use]
pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// Counts the number of path segments in a file path.
#[must_use]
pub fn path_component_count(path: &str) -> usize {
    Path::new(path).components().count()
}

/// Maps an MCP error code to a short string representation for metrics.
#[must_use]
pub fn error_code_to_type(code: rmcp::model::ErrorCode) -> &'static str {
    match code {
        rmcp::model::ErrorCode::PARSE_ERROR => "parse",
        rmcp::model::ErrorCode::INVALID_PARAMS => "invalid_params",
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
    base_dir.join(format!("metrics-{date_str}.jsonl"))
}

async fn cleanup_old_files(base_dir: &Path) {
    let now_days = u32::try_from(unix_ms() / 86_400_000).unwrap_or(u32::MAX);

    let Ok(mut entries) = tokio::fs::read_dir(base_dir).await else {
        return;
    };

    loop {
        match entries.next_entry().await {
            Ok(Some(entry)) => {
                let path = entry.path();
                let file_name = match path.file_name() {
                    Some(n) => n.to_string_lossy().into_owned(),
                    None => continue,
                };

                // Expected format: metrics-YYYY-MM-DD.jsonl
                if !file_name.starts_with("metrics-")
                    || std::path::Path::new(&*file_name)
                        .extension()
                        .is_none_or(|e| !e.eq_ignore_ascii_case("jsonl"))
                {
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
                    let _ = tokio::fs::remove_file(&path).await;
                }
            }
            Ok(None) => break,
            Err(e) => {
                tracing::warn!("error reading metrics directory entry: {e}");
            }
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
    // Compute the proleptic Gregorian day number, then subtract the Unix epoch offset.
    // The subtraction must wrap the full expression; applying .saturating_sub to `doe`
    // alone would underflow for recent dates where doe < 719_468.
    (era * 146_097 + doe).saturating_sub(719_468)
}

/// Returns the current UTC date as a string in YYYY-MM-DD format.
#[must_use]
pub fn current_date_str() -> String {
    let days = u32::try_from(unix_ms() / 86_400_000).unwrap_or(u32::MAX);
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
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- date math tests ---

    #[test]
    fn test_date_to_days_since_epoch_known_dates() {
        // Unix epoch: 1970-01-01 = day 0
        assert_eq!(date_to_days_since_epoch(1970, 1, 1), 0);
        // 2020-01-01: known value 18262 (50 years, accounting for leap years)
        assert_eq!(date_to_days_since_epoch(2020, 1, 1), 18_262);
        // Leap day 2000-02-29: 2000-02-29 is day 11_016
        assert_eq!(date_to_days_since_epoch(2000, 2, 29), 11_016);
    }

    #[test]
    fn test_current_date_str_format() {
        let s = current_date_str();
        assert_eq!(s.len(), 10, "date string must be 10 chars: {s}");
        assert_eq!(s.as_bytes()[4], b'-', "char at index 4 must be '-': {s}");
        assert_eq!(s.as_bytes()[7], b'-', "char at index 7 must be '-': {s}");
        // Sanity: year must parse and be in reasonable range
        let year: u32 = s[0..4].parse().expect("year must be numeric");
        assert!(year >= 2020 && year <= 2100, "unexpected year {year}");
    }

    #[tokio::test]
    async fn test_metrics_writer_batching() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<MetricEvent>();
        let writer = MetricsWriter::new(rx, Some(dir.path().to_path_buf()));

        let make_event = || MetricEvent {
            ts: unix_ms(),
            tool: "analyze_directory",
            duration_ms: 1,
            output_chars: 10,
            param_path_depth: 1,
            max_depth: None,
            result: "ok",
            error_type: None,
            session_id: None,
            seq: None,
        };

        tx.send(make_event()).unwrap();
        tx.send(make_event()).unwrap();
        tx.send(make_event()).unwrap();
        // Drop sender so run() exits after draining
        drop(tx);

        writer.run().await;

        // Exactly 1 .jsonl file must exist with exactly 3 lines
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.eq_ignore_ascii_case("jsonl"))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(entries.len(), 1, "expected exactly 1 .jsonl file");
        let content = std::fs::read_to_string(entries[0].path()).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3, "expected exactly 3 lines; got: {content}");
    }

    #[tokio::test]
    async fn test_cleanup_old_files_deletes_old_keeps_recent() {
        use tempfile::TempDir;

        // Today = 2026-03-28 (hard-coded to match plan; adjust if re-run later)
        // 31 days ago = 2026-02-25; 29 days ago = 2026-02-27
        let dir = TempDir::new().unwrap();
        let old_file = dir.path().join("metrics-2026-02-25.jsonl");
        let recent_file = dir.path().join("metrics-2026-02-27.jsonl");
        std::fs::write(&old_file, "old\n").unwrap();
        std::fs::write(&recent_file, "recent\n").unwrap();

        cleanup_old_files(dir.path()).await;

        assert!(
            !old_file.exists(),
            "31-day-old file should have been deleted"
        );
        assert!(
            recent_file.exists(),
            "29-day-old file should have been kept"
        );
    }

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
