// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
/// Resource limits and configuration for analysis operations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct AnalysisConfig {
    /// Maximum file size in bytes to parse. Files exceeding this limit are skipped.
    /// `None` means no limit.
    pub max_file_bytes: Option<u64>,
    /// Parse timeout in microseconds. Reserved for future use.
    /// `None` means no timeout.
    pub parse_timeout_micros: Option<u64>,
    /// LRU cache capacity for analysis results.
    /// `None` uses the default capacity.
    pub cache_capacity: Option<usize>,
}
