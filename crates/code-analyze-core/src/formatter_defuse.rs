// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
//! Def-use pagination formatting.

use std::fmt::Write;
use std::path::Path;

use crate::formatter::snippet_one_line;

/// Format a page of def-use sites for pagination.
/// Renders a DEF-USE SITES section with WRITES and READS sub-sections.
pub fn format_focused_paginated_defuse(
    paginated_sites: &[crate::types::DefUseSite],
    total: usize,
    symbol: &str,
    offset: usize,
    base_path: Option<&Path>,
    _verbose: bool,
) -> String {
    let mut output = String::new();

    let page_size = paginated_sites.len();
    let start = offset + 1;
    let end = offset + page_size;

    let _ = writeln!(
        output,
        "DEF-USE SITES  {symbol}  ({start}-{end} of {total})"
    );

    // Render writes (Write and WriteRead)
    let write_sites: Vec<_> = paginated_sites
        .iter()
        .filter(|s| {
            matches!(
                s.kind,
                crate::types::DefUseKind::Write | crate::types::DefUseKind::WriteRead
            )
        })
        .collect();

    if !write_sites.is_empty() {
        output.push_str("  WRITES\n");
        for site in write_sites {
            let file_display = strip_base_path(Path::new(&site.file), base_path);
            let scope_str = site
                .enclosing_scope
                .as_ref()
                .map(|s| format!("{}()", s))
                .unwrap_or_default();
            let snippet = snippet_one_line(&site.snippet);
            let _ = writeln!(
                output,
                "    {file_display}:{}  {scope_str}  {snippet}",
                site.line
            );
        }
    }

    // Render reads
    let read_sites: Vec<_> = paginated_sites
        .iter()
        .filter(|s| matches!(s.kind, crate::types::DefUseKind::Read))
        .collect();

    if !read_sites.is_empty() {
        output.push_str("  READS\n");
        for site in read_sites {
            let file_display = strip_base_path(Path::new(&site.file), base_path);
            let scope_str = site
                .enclosing_scope
                .as_ref()
                .map(|s| format!("{}()", s))
                .unwrap_or_default();
            let snippet = snippet_one_line(&site.snippet);
            let _ = writeln!(
                output,
                "    {file_display}:{}  {scope_str}  {snippet}",
                site.line
            );
        }
    }

    output
}

/// Strip base path from a full file path for display.
fn strip_base_path(path: &Path, base_path: Option<&Path>) -> String {
    if let Some(base) = base_path
        && let Ok(rel) = path.strip_prefix(base)
    {
        return rel.to_string_lossy().into_owned();
    }
    path.to_string_lossy().into_owned()
}
