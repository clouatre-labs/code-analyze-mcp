//! Path completion support for file and directory paths.
//!
//! Provides completion suggestions for partial paths within a directory tree,
//! respecting .gitignore and .ignore files.

use crate::cache::AnalysisCache;
use ignore::WalkBuilder;
use std::path::Path;
use tracing::instrument;

/// Get path completions for a given prefix within a root directory.
/// Uses ignore crate with standard filters to respect .gitignore.
/// Returns matching file and directory paths up to 100 results.
#[instrument(skip_all, fields(prefix = %prefix))]
pub fn path_completions(root: &Path, prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }

    // Determine the search directory and filename prefix
    let (search_dir, name_prefix) = if let Some(last_slash) = prefix.rfind('/') {
        let dir_part = &prefix[..=last_slash];
        let name_part = &prefix[last_slash + 1..];
        let full_path = root.join(dir_part);
        (full_path, name_part.to_string())
    } else {
        (root.to_path_buf(), prefix.to_string())
    };

    // If search directory doesn't exist, return empty
    if !search_dir.exists() {
        return Vec::new();
    }

    let mut results = Vec::new();

    // Walk with depth 1 to get immediate children
    let mut builder = WalkBuilder::new(&search_dir);
    builder
        .hidden(true)
        .standard_filters(true)
        .max_depth(Some(1));

    for result in builder.build() {
        if results.len() >= 100 {
            break;
        }

        match result {
            Ok(entry) => {
                let path = entry.path();
                // Skip the root directory itself
                if path == search_dir {
                    continue;
                }

                // Get the filename
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                    && file_name.starts_with(&name_prefix)
                {
                    // Construct relative path from root
                    if let Ok(rel_path) = path.strip_prefix(root) {
                        let rel_str = rel_path.to_string_lossy().to_string();
                        results.push(rel_str);
                    }
                }
            }
            Err(_) => {
                // Skip unreadable entries
                continue;
            }
        }
    }

    results
}

/// Get symbol completions (function and class names) for a given file path.
/// Looks up cached FileAnalysisOutput and extracts matching symbols.
/// Returns matching function and class names up to 100 results.
#[instrument(skip(cache), fields(path = %path.display(), prefix = %prefix))]
pub fn symbol_completions(cache: &AnalysisCache, path: &Path, prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }

    // Get file metadata for cache key
    let cache_key = match std::fs::metadata(path) {
        Ok(meta) => match meta.modified() {
            Ok(mtime) => crate::cache::CacheKey {
                path: path.to_path_buf(),
                modified: mtime,
                mode: crate::types::AnalysisMode::FileDetails,
            },
            Err(_) => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };

    // Look up in cache
    let cached = match cache.get(&cache_key) {
        Some(output) => output,
        None => return Vec::new(),
    };

    let mut results = Vec::new();

    // Extract function names matching prefix
    for func in &cached.semantic.functions {
        if results.len() >= 100 {
            break;
        }
        if func.name.starts_with(prefix) {
            results.push(func.name.clone());
        }
    }

    // Extract class names matching prefix
    for class in &cached.semantic.classes {
        if results.len() >= 100 {
            break;
        }
        if class.name.starts_with(prefix) {
            results.push(class.name.clone());
        }
    }

    results
}
