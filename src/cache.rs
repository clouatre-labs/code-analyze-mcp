use crate::analyze::FileAnalysisOutput;
use crate::types::AnalysisMode;
use lru::LruCache;
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

/// Recover from a poisoned mutex by clearing the cache.
/// On poison, creates a new empty cache and returns the recovery value.
fn lock_or_recover<T, F>(
    mutex: &Mutex<LruCache<CacheKey, Arc<FileAnalysisOutput>>>,
    recovery: F,
) -> T
where
    F: FnOnce(&mut LruCache<CacheKey, Arc<FileAnalysisOutput>>) -> T,
{
    match mutex.lock() {
        Ok(mut guard) => recovery(&mut guard),
        Err(poisoned) => {
            let cache_size = NonZeroUsize::new(100).unwrap();
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
}

impl AnalysisCache {
    /// Create a new cache with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        let cache_size = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
        }
    }

    /// Get a cached analysis result if it exists.
    #[instrument(skip(self), fields(path = ?key.path))]
    pub fn get(&self, key: &CacheKey) -> Option<Arc<FileAnalysisOutput>> {
        lock_or_recover(&self.cache, |guard| {
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
        lock_or_recover(&self.cache, |guard| {
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
}

impl Clone for AnalysisCache {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
        }
    }
}
