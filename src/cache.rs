//! LRU cache for analysis results indexed by path, modification time, and mode.
//!
//! Provides thread-safe, capacity-bounded caching of file analysis outputs using LRU eviction.
//! Recovers gracefully from poisoned mutex conditions.

use crate::analyze::{AnalysisOutput, FileAnalysisOutput};
use crate::traversal::WalkEntry;
use crate::types::AnalysisMode;
use lru::LruCache;
use rayon::prelude::*;
use std::fs;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tracing::{debug, instrument};

const DIR_CACHE_CAPACITY: usize = 20;

/// Cache key combining path, modification time, and analysis mode.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub mode: AnalysisMode,
}

/// Cache key for directory analysis combining file mtimes, mode, and `max_depth`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DirectoryCacheKey {
    files: Vec<(PathBuf, SystemTime)>,
    mode: AnalysisMode,
    max_depth: Option<u32>,
}

impl DirectoryCacheKey {
    /// Build a cache key from walk entries, capturing mtime for each file.
    /// Files are sorted by path for deterministic hashing.
    /// Directories are filtered out; only file entries are processed.
    /// Metadata collection is parallelized using rayon.
    #[must_use]
    pub fn from_entries(entries: &[WalkEntry], max_depth: Option<u32>, mode: AnalysisMode) -> Self {
        let mut files: Vec<(PathBuf, SystemTime)> = entries
            .par_iter()
            .filter(|e| !e.is_dir)
            .map(|e| {
                let mtime = fs::metadata(&e.path)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                (e.path.clone(), mtime)
            })
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            files,
            mode,
            max_depth,
        }
    }
}

/// Recover from a poisoned mutex by clearing the cache.
/// On poison, creates a new empty cache and returns the recovery value.
fn lock_or_recover<K, V, T, F>(mutex: &Mutex<LruCache<K, V>>, capacity: usize, recovery: F) -> T
where
    K: std::hash::Hash + Eq,
    F: FnOnce(&mut LruCache<K, V>) -> T,
{
    match mutex.lock() {
        Ok(mut guard) => recovery(&mut guard),
        Err(poisoned) => {
            let cache_size = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
            let new_cache = LruCache::new(cache_size);
            let mut guard = poisoned.into_inner();
            *guard = new_cache;
            recovery(&mut guard)
        }
    }
}

/// LRU cache for file analysis results with mutex protection.
pub struct AnalysisCache {
    file_capacity: usize,
    cache: Arc<Mutex<LruCache<CacheKey, Arc<FileAnalysisOutput>>>>,
    directory_cache: Arc<Mutex<LruCache<DirectoryCacheKey, Arc<AnalysisOutput>>>>,
}

impl AnalysisCache {
    /// Create a new cache with the specified capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let file_capacity = capacity.max(1);
        let cache_size = NonZeroUsize::new(file_capacity).unwrap();
        let dir_cache_size = NonZeroUsize::new(DIR_CACHE_CAPACITY).unwrap();
        Self {
            file_capacity,
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            directory_cache: Arc::new(Mutex::new(LruCache::new(dir_cache_size))),
        }
    }

    /// Get a cached analysis result if it exists.
    #[instrument(skip(self), fields(path = ?key.path))]
    pub fn get(&self, key: &CacheKey) -> Option<Arc<FileAnalysisOutput>> {
        lock_or_recover(&self.cache, self.file_capacity, |guard| {
            let result = guard.get(key).cloned();
            let cache_size = guard.len();
            if let Some(v) = result {
                debug!(cache_event = "hit", cache_size = cache_size, path = ?key.path);
                Some(v)
            } else {
                debug!(cache_event = "miss", cache_size = cache_size, path = ?key.path);
                None
            }
        })
    }

    /// Store an analysis result in the cache.
    #[instrument(skip(self, value), fields(path = ?key.path))]
    // public API; callers expect owned semantics
    #[allow(clippy::needless_pass_by_value)]
    pub fn put(&self, key: CacheKey, value: Arc<FileAnalysisOutput>) {
        lock_or_recover(&self.cache, self.file_capacity, |guard| {
            let push_result = guard.push(key.clone(), value);
            let cache_size = guard.len();
            match push_result {
                None => {
                    debug!(cache_event = "insert", cache_size = cache_size, path = ?key.path);
                }
                Some((returned_key, _)) => {
                    if returned_key == key {
                        debug!(cache_event = "update", cache_size = cache_size, path = ?key.path);
                    } else {
                        debug!(cache_event = "eviction", cache_size = cache_size, path = ?key.path, evicted_path = ?returned_key.path);
                    }
                }
            }
        });
    }

    /// Get a cached directory analysis result if it exists.
    #[instrument(skip(self))]
    pub fn get_directory(&self, key: &DirectoryCacheKey) -> Option<Arc<AnalysisOutput>> {
        lock_or_recover(&self.directory_cache, DIR_CACHE_CAPACITY, |guard| {
            let result = guard.get(key).cloned();
            let cache_size = guard.len();
            if let Some(v) = result {
                debug!(cache_event = "hit", cache_size = cache_size);
                Some(v)
            } else {
                debug!(cache_event = "miss", cache_size = cache_size);
                None
            }
        })
    }

    /// Store a directory analysis result in the cache.
    #[instrument(skip(self, value))]
    pub fn put_directory(&self, key: DirectoryCacheKey, value: Arc<AnalysisOutput>) {
        lock_or_recover(&self.directory_cache, DIR_CACHE_CAPACITY, |guard| {
            let push_result = guard.push(key, value);
            let cache_size = guard.len();
            match push_result {
                None => {
                    debug!(cache_event = "insert", cache_size = cache_size);
                }
                Some((_, _)) => {
                    debug!(cache_event = "eviction", cache_size = cache_size);
                }
            }
        });
    }
}

impl Clone for AnalysisCache {
    fn clone(&self) -> Self {
        Self {
            file_capacity: self.file_capacity,
            cache: Arc::clone(&self.cache),
            directory_cache: Arc::clone(&self.directory_cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_entries_skips_dirs() {
        // Arrange: create a real temp dir and a real temp file for hermetic isolation.
        let dir = tempfile::tempdir().expect("tempdir");
        let file = tempfile::NamedTempFile::new_in(dir.path()).expect("tempfile");
        let file_path = file.path().to_path_buf();

        let entries = vec![
            WalkEntry {
                path: dir.path().to_path_buf(),
                depth: 0,
                is_dir: true,
                is_symlink: false,
                symlink_target: None,
            },
            WalkEntry {
                path: file_path.clone(),
                depth: 0,
                is_dir: false,
                is_symlink: false,
                symlink_target: None,
            },
        ];

        // Act: build cache key from entries
        let key = DirectoryCacheKey::from_entries(&entries, None, AnalysisMode::Overview);

        // Assert: only the file entry should be in the cache key
        // The directory entry should be filtered out
        assert_eq!(key.files.len(), 1);
        assert_eq!(key.files[0].0, file_path);
    }
}
