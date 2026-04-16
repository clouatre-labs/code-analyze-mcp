// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
//! Directory traversal with .gitignore support.
//!
//! Provides recursive directory walking with automatic filtering based on `.gitignore` and `.ignore` files.
//! Uses the `ignore` crate for cross-platform, efficient file system traversal.

use ignore::WalkBuilder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
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
#[non_exhaustive]
pub enum TraversalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("internal concurrency error: {0}")]
    Internal(String),
    #[error("git error: {0}")]
    GitError(String),
}

/// Walk a directory with support for `.gitignore` and `.ignore`.
/// `max_depth=0` maps to unlimited recursion (None), positive values limit depth.
/// The returned entries are sorted lexicographically by path.
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
                let is_dir = entry.file_type().is_some_and(|ft| ft.is_dir());
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
                let Ok(mut guard) = entries.lock() else {
                    tracing::debug!("mutex poisoned in parallel walker, skipping entry");
                    return ignore::WalkState::Skip;
                };
                guard.push(walk_entry);
                ignore::WalkState::Continue
            }
            Err(e) => {
                tracing::warn!(error = %e, "skipping unreadable entry");
                ignore::WalkState::Continue
            }
        })
    });

    let mut entries = Arc::try_unwrap(entries)
        .map_err(|_| {
            TraversalError::Internal("arc unwrap failed: strong references still live".to_string())
        })?
        .into_inner()
        .map_err(|_| TraversalError::Internal("mutex poisoned".to_string()))?;

    let dir_count = entries.iter().filter(|e| e.is_dir).count();
    let file_count = entries.iter().filter(|e| !e.is_dir).count();

    tracing::debug!(
        entries = entries.len(),
        dirs = dir_count,
        files = file_count,
        duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        "walk complete"
    );

    // Restore sort contract: walk_parallel does not guarantee order.
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// Return the set of absolute paths changed relative to `git_ref` in the repository
/// containing `dir`. Invokes git without shell interpolation.
///
/// # Errors
/// Returns [`TraversalError::GitError`] when:
/// - `git` is not on PATH (distinct message)
/// - `dir` is not inside a git repository
pub fn changed_files_from_git_ref(
    dir: &Path,
    git_ref: &str,
) -> Result<HashSet<PathBuf>, TraversalError> {
    // Resolve the git repository root so that relative paths from `git diff` can
    // be anchored to an absolute base.
    let root_out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TraversalError::GitError("git not found on PATH".to_string())
            } else {
                TraversalError::GitError(format!("failed to run git: {e}"))
            }
        })?;

    if !root_out.status.success() {
        let stderr = String::from_utf8_lossy(&root_out.stderr);
        return Err(TraversalError::GitError(format!(
            "not a git repository: {stderr}"
        )));
    }

    let root = PathBuf::from(String::from_utf8_lossy(&root_out.stdout).trim().to_string());

    // Run `git diff --name-only <git_ref>` to get changed files relative to root.
    let diff_out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("diff")
        .arg("--name-only")
        .arg(git_ref)
        .output()
        .map_err(|e| TraversalError::GitError(format!("failed to run git diff: {e}")))?;

    if !diff_out.status.success() {
        let stderr = String::from_utf8_lossy(&diff_out.stderr);
        return Err(TraversalError::GitError(format!(
            "git diff failed: {stderr}"
        )));
    }

    let changed: HashSet<PathBuf> = String::from_utf8_lossy(&diff_out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| root.join(l))
        .collect();

    Ok(changed)
}

/// Filter walk entries to only those that are either changed files or ancestor directories
/// of changed files. This preserves the tree structure while limiting analysis scope.
#[must_use]
pub fn filter_entries_by_git_ref(
    entries: Vec<WalkEntry>,
    changed: &HashSet<PathBuf>,
    root: &Path,
) -> Vec<WalkEntry> {
    entries
        .into_iter()
        .filter(|e| {
            if e.is_dir {
                // Keep directory if any changed file is underneath it.
                changed.iter().any(|c| c.starts_with(&e.path)) || e.path == root
            } else {
                changed.contains(&e.path)
            }
        })
        .collect()
}

/// Compute files-per-depth-1-subdirectory counts from an already-collected entry list.
/// Returns a Vec of (depth-1 path, file count) sorted by path.
/// Only counts file entries (not directories); skips entries containing `EXCLUDED_DIRS` components.
/// Output Vec is sorted by construction (entries are pre-sorted by path).
#[must_use]
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
        let Ok(rel) = entry.path.strip_prefix(root) else {
            continue;
        };
        if let Some(first) = rel.components().next() {
            let key = root.join(first);
            match counts.last_mut() {
                Some(last) if last.0 == key => last.1 += 1,
                _ => counts.push((key, 1)),
            }
        }
    }
    counts
}
