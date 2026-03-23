//! LRU cache for analysis results indexed by path, modification time, and mode.
//!
//! Provides thread-safe, capacity-bounded caching of file analysis outputs using LRU eviction.
//! Recovers gracefully from poisoned mutex conditions.

use crate::analyze::{AnalysisOutput, FileAnalysisOutput};
use crate::traversal::WalkEntry;
use crate::types::AnalysisMode;
use lru::LruCache;
use std::fs;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tracing::{debug, instrument};

/// Cache key combining path, modification time, and analysis mode.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CacheKey {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub mode: AnalysisMode,
}

/// Cache key for directory analysis combining file mtimes, mode, and max_depth.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DirectoryCacheKey {
    files: Vec<(PathBuf, SystemTime)>,
    mode: AnalysisMode,
    max_depth: Option<u32>,
}

impl DirectoryCacheKey {
    /// Build a cache key from walk entries, capturing mtime for each file.
    /// Files are sorted by path for deterministic hashing.
    pub fn from_entries(entries: &[WalkEntry], max_depth: Option<u32>, mode: AnalysisMode) -> Self {
        let mut files: Vec<(PathBuf, SystemTime)> = entries
            .iter()
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
    cache: Arc<Mutex<LruCache<CacheKey, Arc<FileAnalysisOutput>>>>,
    directory_cache: Arc<Mutex<LruCache<DirectoryCacheKey, Arc<AnalysisOutput>>>>,
}

impl AnalysisCache {
    /// Create a new cache with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        let cache_size = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        let dir_cache_size = NonZeroUsize::new(20).unwrap();
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            directory_cache: Arc::new(Mutex::new(LruCache::new(dir_cache_size))),
        }
    }

    /// Get a cached analysis result if it exists.
    #[instrument(skip(self), fields(path = ?key.path))]
    pub fn get(&self, key: &CacheKey) -> Option<Arc<FileAnalysisOutput>> {
        lock_or_recover(&self.cache, 100, |guard| {
            let result = guard.get(key).cloned();
            let cache_size = guard.len();
            match result {
                Some(v) => {
                    debug!(cache_event = "hit", cache_size = cache_size, path = ?key.path);
                    Some(v)
                }
                None => {
                    debug!(cache_event = "miss", cache_size = cache_size, path = ?key.path);
                    None
                }
            }
        })
    }

    /// Store an analysis result in the cache.
    #[instrument(skip(self, value), fields(path = ?key.path))]
    pub fn put(&self, key: CacheKey, value: Arc<FileAnalysisOutput>) {
        lock_or_recover(&self.cache, 100, |guard| {
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
        lock_or_recover(&self.directory_cache, 20, |guard| {
            let result = guard.get(key).cloned();
            let cache_size = guard.len();
            match result {
                Some(v) => {
                    debug!(cache_event = "hit", cache_size = cache_size);
                    Some(v)
                }
                None => {
                    debug!(cache_event = "miss", cache_size = cache_size);
                    None
                }
            }
        })
    }

    /// Store a directory analysis result in the cache.
    #[instrument(skip(self, value))]
    pub fn put_directory(&self, key: DirectoryCacheKey, value: Arc<AnalysisOutput>) {
        lock_or_recover(&self.directory_cache, 20, |guard| {
            let push_result = guard.push(key.clone(), value);
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
            cache: Arc::clone(&self.cache),
            directory_cache: Arc::clone(&self.directory_cache),
        }
    }
}
