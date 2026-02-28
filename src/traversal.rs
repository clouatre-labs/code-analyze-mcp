use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct WalkEntry {
    pub path: PathBuf,
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

/// Walk a directory with support for .gitignore and .gooseignore.
/// max_depth=0 maps to unlimited recursion (None), positive values limit depth.
#[instrument(skip_all, fields(path = %root.display(), max_depth))]
pub fn walk_directory(
    root: &Path,
    max_depth: Option<u32>,
) -> Result<Vec<WalkEntry>, TraversalError> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(true)
        .standard_filters(true)
        .add_custom_ignore_filename(".gooseignore");

    // Map max_depth: 0 = unlimited (None), positive = Some(n)
    if let Some(depth) = max_depth {
        if depth > 0 {
            builder.max_depth(Some(depth as usize));
        }
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

    Ok(entries)
}
