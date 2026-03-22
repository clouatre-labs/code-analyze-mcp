//! Directory traversal with .gitignore support.
//!
//! Provides recursive directory walking with automatic filtering based on `.gitignore` and `.ignore` files.
//! Uses the `ignore` crate for cross-platform, efficient file system traversal.

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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

    let entries = Arc::new(Mutex::new(Vec::new()));
    let entries_clone = Arc::clone(&entries);

    builder.build_parallel().run(move || {
        let entries = Arc::clone(&entries_clone);
        Box::new(move |result| match result {
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

                let walk_entry = WalkEntry {
                    path,
                    depth,
                    is_dir,
                    is_symlink,
                    symlink_target,
                };
                entries.lock().unwrap().push(walk_entry);
                ignore::WalkState::Continue
            }
            Err(e) => {
                tracing::warn!(error = %e, "skipping unreadable entry");
                ignore::WalkState::Continue
            }
        })
    });

    let mut entries = Arc::try_unwrap(entries)
        .map_err(|_| TraversalError::Io(std::io::Error::other("arc unwrap failed")))?
        .into_inner()
        .map_err(|_| TraversalError::Io(std::io::Error::other("mutex poisoned")))?;

    let dir_count = entries.iter().filter(|e| e.is_dir).count();
    let file_count = entries.iter().filter(|e| !e.is_dir).count();

    tracing::debug!(
        entries = entries.len(),
        dirs = dir_count,
        files = file_count,
        duration_ms = start.elapsed().as_millis() as u64,
        "walk complete"
    );

    // Restore sort contract: walk_parallel does not guarantee order.
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// Returns the depth-1 component under `root` for a given `path`,
/// or `root` itself if `path` == `root`.
fn depth1_prefix(root: &Path, path: &Path) -> PathBuf {
    let rel = path.strip_prefix(root).unwrap_or(path);
    match rel.components().next() {
        Some(c) => root.join(c),
        None => root.to_path_buf(),
    }
}

/// Compute files-per-depth-1-subdirectory counts from an already-collected entry list.
/// Returns a Vec of (depth-1 path, file count) sorted by path.
/// Only counts file entries (not directories); skips entries containing EXCLUDED_DIRS components.
/// Output Vec is sorted by construction (entries are pre-sorted by path).
pub fn subtree_counts_from_entries(root: &Path, entries: &[WalkEntry]) -> Vec<(PathBuf, usize)> {
    let mut counts: Vec<(PathBuf, usize)> = Vec::new();
    for entry in entries {
        if entry.is_dir {
            continue;
        }
        // Skip entries whose path components contain EXCLUDED_DIRS
        if entry.path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            crate::EXCLUDED_DIRS.contains(&s.as_ref())
        }) {
            continue;
        }
        let rel = match entry.path.strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.components().next().is_some() {
            let key = depth1_prefix(root, &entry.path);
            match counts.last_mut() {
                Some(last) if last.0 == key => last.1 += 1,
                _ => counts.push((key, 1)),
            }
        }
    }
    counts
}
