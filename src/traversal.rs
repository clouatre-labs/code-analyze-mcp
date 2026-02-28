use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub struct DirEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

pub struct WalkOptions {
    /// Maximum recursion depth. 0 means unlimited.
    pub max_depth: usize,
}

pub fn walk_directory(root: &Path, options: &WalkOptions) -> Vec<DirEntry> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(true)
        .git_ignore(true)
        .require_git(false)
        .follow_links(false)
        .add_custom_ignore_filename(".gooseignore");

    if options.max_depth > 0 {
        builder.max_depth(Some(options.max_depth));
    }

    let mut entries = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                tracing::debug!(error = %err, "Skipping unreadable directory entry");
                continue;
            }
        };

        if entry.depth() == 0 {
            continue; // skip the root itself
        }

        let path = entry.path().to_path_buf();
        let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        let is_symlink = entry.path_is_symlink();
        let symlink_target = if is_symlink {
            std::fs::read_link(&path).ok()
        } else {
            None
        };
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        entries.push(DirEntry {
            path,
            relative_path,
            depth: entry.depth(),
            is_dir,
            is_symlink,
            symlink_target,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_walk_directory_basic() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let entries = walk_directory(&src, &WalkOptions { max_depth: 1 });
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.relative_path.to_str() == Some("lib.rs")));
    }

    #[test]
    fn test_walk_directory_max_depth_zero_unlimited() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let limited = walk_directory(&src, &WalkOptions { max_depth: 1 });
        let unlimited = walk_directory(&src, &WalkOptions { max_depth: 0 });
        assert!(unlimited.len() >= limited.len());
    }

    #[test]
    fn test_walk_directory_skips_hidden() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let entries = walk_directory(root, &WalkOptions { max_depth: 1 });
        assert!(
            entries
                .iter()
                .all(|e| !e.relative_path.to_str().unwrap_or("").starts_with('.')),
            "Hidden entries should be skipped"
        );
    }

    #[test]
    fn test_walk_directory_gitignore_respected() {
        let tmp = std::env::temp_dir().join("ts_walk_test_gitignore");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join(".gitignore"), "ignored.rs\n").unwrap();
        fs::write(tmp.join("ignored.rs"), "fn ignored() {}").unwrap();
        fs::write(tmp.join("kept.rs"), "fn kept() {}").unwrap();

        let entries = walk_directory(&tmp, &WalkOptions { max_depth: 1 });
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_str().unwrap_or("").to_string())
            .collect();

        assert!(names.contains(&"kept.rs".to_string()));
        assert!(!names.contains(&"ignored.rs".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_directory_gooseignore_respected() {
        let tmp = std::env::temp_dir().join("ts_walk_test_gooseignore");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join(".gooseignore"), "goose_ignored.rs\n").unwrap();
        fs::write(tmp.join("goose_ignored.rs"), "fn goose() {}").unwrap();
        fs::write(tmp.join("kept.rs"), "fn kept() {}").unwrap();

        let entries = walk_directory(&tmp, &WalkOptions { max_depth: 1 });
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_str().unwrap_or("").to_string())
            .collect();

        assert!(names.contains(&"kept.rs".to_string()));
        assert!(
            !names.contains(&"goose_ignored.rs".to_string()),
            ".gooseignore patterns must be respected"
        );

        let _ = fs::remove_dir_all(&tmp);
    }
}
