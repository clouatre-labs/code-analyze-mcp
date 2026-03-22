//! Directory traversal with .gitignore support.
//!
//! Provides recursive directory walking with automatic filtering based on `.gitignore` and `.ignore` files.
//! Uses the `ignore` crate for cross-platform, efficient file system traversal.

use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct WalkEntry {
    pub path: PathBuf,
    /// Depth in the directory tree (0 = root).
    pub depth: usize,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum TraversalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Walk a directory with support for .gitignore and .ignore.
/// max_depth=0 maps to unlimited recursion (None), positive values limit depth.
#[instrument(skip_all, fields(path = %root.display(), max_depth))]
pub fn walk_directory(
    root: &Path,
    max_depth: Option<u32>,
) -> Result<Vec<WalkEntry>, TraversalError> {
    let start = Instant::now();
    let mut builder = WalkBuilder::new(root);
    builder.hidden(true).standard_filters(true);

    // Map max_depth: 0 = unlimited (None), positive = Some(n)
    if let Some(depth) = max_depth
        && depth > 0
    {
        builder.max_depth(Some(depth as usize));
    }

    let mut entries = Vec::new();

    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                let depth = entry.depth();
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let is_symlink = entry.path_is_symlink();

                let symlink_target = if is_symlink {
                    std::fs::read_link(&path).ok()
                } else {
                    None
                };

                entries.push(WalkEntry {
                    path,
                    depth,
                    is_dir,
                    is_symlink,
                    symlink_target,
                });
            }
            Err(e) => {
                tracing::warn!(error = %e, "skipping unreadable entry");
                continue;
            }
        }
    }

    let dir_count = entries.iter().filter(|e| e.is_dir).count();
    let file_count = entries.iter().filter(|e| !e.is_dir).count();

    tracing::debug!(
        entries = entries.len(),
        dirs = dir_count,
        files = file_count,
        duration_ms = start.elapsed().as_millis() as u64,
        "walk complete"
    );

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// Counts files per depth-1 subdirectory of `root` using an unbounded walk.
/// Uses identical WalkBuilder filters as `walk_directory` (hidden + standard_filters).
/// Returns a map from each depth-1 child path to its total descendant file count.
/// Does not allocate WalkEntry structs; only counts.
pub fn count_files_by_dir(root: &Path) -> Result<HashMap<PathBuf, usize>, TraversalError> {
    let mut counts: HashMap<PathBuf, usize> = HashMap::new();
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .standard_filters(true)
        .build();
    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("count_files_by_dir walk error: {}", e);
                continue;
            }
        };
        // Skip directories; only count files
        let ft = entry.file_type();
        if ft.map(|f| f.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        // Skip entries whose path components contain EXCLUDED_DIRS
        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            crate::EXCLUDED_DIRS.contains(&s.as_ref())
        }) {
            continue;
        }
        // Find the depth-1 ancestor of this file relative to root
        let rel = match path.strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        // First component of the relative path is the depth-1 child dir
        if let Some(first) = rel.components().next() {
            let depth1 = root.join(first);
            *counts.entry(depth1).or_insert(0) += 1;
        }
    }
    Ok(counts)
}
