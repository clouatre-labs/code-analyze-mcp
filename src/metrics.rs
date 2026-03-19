use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEvent {
    pub ts: u64,
    pub tool: &'static str,
    pub duration_ms: u64,
    pub output_bytes: usize,
    pub param_path_depth: usize,
    pub max_depth: Option<u32>,
    pub result: &'static str,
    pub error_type: Option<String>,
}

#[derive(Clone)]
pub struct MetricsSender(pub tokio::sync::mpsc::UnboundedSender<MetricEvent>);

impl MetricsSender {
    pub fn send(&self, event: MetricEvent) {
        let _ = self.0.send(event);
    }
}

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

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn path_component_count(path: &str) -> usize {
    Path::new(path).components().count()
}

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
            output_bytes: 100,
            param_path_depth: 3,
            max_depth: Some(2),
            result: "ok",
            error_type: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("analyze_directory"));
        assert!(json.contains(r#""result":"ok""#));
    }
}
