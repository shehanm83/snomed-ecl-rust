//! Query result caching for ECL execution.
//!
//! Provides an LRU cache with TTL expiration for caching ECL query results.
//! Thread-safe using `Mutex` for LRU operations.

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lru::LruCache;
use snomed_ecl::SctId;

use crate::config::CacheConfig;

/// A cached query result with expiration tracking.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached concept IDs.
    result: HashSet<SctId>,
    /// When this entry was created.
    created_at: Instant,
}

impl CacheEntry {
    /// Creates a new cache entry with the current timestamp.
    fn new(result: HashSet<SctId>) -> Self {
        Self {
            result,
            created_at: Instant::now(),
        }
    }

    /// Checks if this entry has expired.
    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

/// Thread-safe LRU cache with TTL expiration for ECL query results.
///
/// # Features
///
/// - **LRU Eviction**: When the cache is full, the least recently used entry is evicted.
/// - **TTL Expiration**: Entries automatically expire after the configured time-to-live.
/// - **Thread-Safe**: Uses `Mutex` for safe concurrent access.
///
/// # Example
///
/// ```ignore
/// use snomed_ecl_executor::cache::QueryCache;
/// use snomed_ecl_executor::CacheConfig;
/// use std::collections::HashSet;
///
/// let config = CacheConfig::default();
/// let cache = QueryCache::new(config);
///
/// // Store a result
/// let result: HashSet<u64> = [100, 200, 300].into_iter().collect();
/// cache.set("<< 73211009".to_string(), result.clone());
///
/// // Retrieve the result
/// if let Some(cached) = cache.get("<< 73211009") {
///     assert_eq!(cached, result);
/// }
/// ```
pub struct QueryCache {
    /// The LRU cache wrapped in a mutex for thread-safety.
    inner: Mutex<LruCache<String, CacheEntry>>,
    /// Time-to-live for cache entries.
    ttl: Duration,
    /// Whether to cache intermediate results.
    cache_intermediates: bool,
}

impl QueryCache {
    /// Creates a new query cache with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Cache configuration specifying size, TTL, and behavior.
    pub fn new(config: CacheConfig) -> Self {
        let capacity = NonZeroUsize::new(config.max_entries.max(1)).unwrap();
        Self {
            inner: Mutex::new(LruCache::new(capacity)),
            ttl: config.ttl,
            cache_intermediates: config.cache_intermediates,
        }
    }

    /// Creates a cache with custom capacity and TTL.
    ///
    /// # Arguments
    ///
    /// * `max_entries` - Maximum number of cached entries.
    /// * `ttl` - Time-to-live for cache entries.
    pub fn with_capacity(max_entries: usize, ttl: Duration) -> Self {
        let capacity = NonZeroUsize::new(max_entries.max(1)).unwrap();
        Self {
            inner: Mutex::new(LruCache::new(capacity)),
            ttl,
            cache_intermediates: true,
        }
    }

    /// Gets a cached result by key.
    ///
    /// Returns `None` if:
    /// - The key doesn't exist in the cache
    /// - The entry has expired (TTL exceeded)
    ///
    /// On cache hit, the entry is promoted to most-recently-used.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key (typically the normalized ECL expression).
    ///
    /// # Returns
    ///
    /// `Some(HashSet<SctId>)` if a valid (non-expired) entry exists, `None` otherwise.
    pub fn get(&self, key: &str) -> Option<HashSet<SctId>> {
        let mut cache = self.inner.lock().ok()?;

        // Check if entry exists and get it (this promotes to MRU)
        if let Some(entry) = cache.get(key) {
            if entry.is_expired(self.ttl) {
                // Entry expired, remove it
                cache.pop(key);
                return None;
            }
            return Some(entry.result.clone());
        }

        None
    }

    /// Stores a result in the cache.
    ///
    /// If the cache is full, the least recently used entry is automatically evicted.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key (typically the normalized ECL expression).
    /// * `result` - The concept IDs to cache.
    pub fn set(&self, key: String, result: HashSet<SctId>) {
        if let Ok(mut cache) = self.inner.lock() {
            let entry = CacheEntry::new(result);
            cache.put(key, entry);
        }
    }

    /// Checks if a key exists in the cache (without affecting LRU order).
    ///
    /// Note: This doesn't check for expiration to avoid modifying the cache.
    pub fn contains(&self, key: &str) -> bool {
        match self.inner.lock() {
            Ok(cache) => cache.contains(key),
            _ => false,
        }
    }

    /// Returns the number of entries currently in the cache.
    ///
    /// Note: This may include expired entries that haven't been cleaned up yet.
    pub fn len(&self) -> usize {
        match self.inner.lock() {
            Ok(cache) => cache.len(),
            _ => 0,
        }
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all entries from the cache.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.inner.lock() {
            cache.clear();
        }
    }

    /// Removes expired entries from the cache.
    ///
    /// This is called lazily during normal operations, but can be called
    /// explicitly to free memory.
    pub fn cleanup_expired(&self) {
        if let Ok(mut cache) = self.inner.lock() {
            let ttl = self.ttl;
            // Collect expired keys
            let expired_keys: Vec<String> = cache
                .iter()
                .filter(|(_, entry)| entry.is_expired(ttl))
                .map(|(key, _)| key.clone())
                .collect();

            // Remove expired entries
            for key in expired_keys {
                cache.pop(&key);
            }
        }
    }

    /// Returns whether intermediate results should be cached.
    pub fn should_cache_intermediates(&self) -> bool {
        self.cache_intermediates
    }

    /// Returns cache statistics.
    pub fn stats(&self) -> CacheStats {
        match self.inner.lock() {
            Ok(cache) => {
                let total = cache.len();
                let expired = cache
                    .iter()
                    .filter(|(_, entry)| entry.is_expired(self.ttl))
                    .count();

                CacheStats {
                    total_entries: total,
                    expired_entries: expired,
                    valid_entries: total.saturating_sub(expired),
                }
            }
            _ => CacheStats::default(),
        }
    }
}

impl std::fmt::Debug for QueryCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        f.debug_struct("QueryCache")
            .field("entries", &stats.total_entries)
            .field("ttl", &self.ttl)
            .field("cache_intermediates", &self.cache_intermediates)
            .finish()
    }
}

/// Statistics about the cache state.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of entries in the cache.
    pub total_entries: usize,
    /// Number of expired entries (not yet cleaned up).
    pub expired_entries: usize,
    /// Number of valid (non-expired) entries.
    pub valid_entries: usize,
}

/// Normalizes an ECL expression string for consistent cache keys.
///
/// This ensures that equivalent ECL expressions map to the same cache key:
/// - Removes extra whitespace
/// - Converts to lowercase for operators
/// - Trims leading/trailing whitespace
///
/// # Example
///
/// ```ignore
/// assert_eq!(normalize_cache_key("<<  73211009"), "<< 73211009");
/// assert_eq!(normalize_cache_key("< 100  AND < 200"), "< 100 and < 200");
/// ```
pub fn normalize_cache_key(ecl: &str) -> String {
    // Remove extra whitespace and normalize
    let mut result = String::with_capacity(ecl.len());
    let mut prev_was_space = true; // Start true to trim leading spaces

    for ch in ecl.chars() {
        if ch.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(ch);
            prev_was_space = false;
        }
    }

    // Trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn create_test_cache(max_entries: usize, ttl_secs: u64) -> QueryCache {
        QueryCache::with_capacity(max_entries, Duration::from_secs(ttl_secs))
    }

    fn create_result(ids: &[SctId]) -> HashSet<SctId> {
        ids.iter().copied().collect()
    }

    // Basic operations tests

    #[test]
    fn test_cache_new() {
        let config = CacheConfig::default();
        let cache = QueryCache::new(config);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_set_get() {
        let cache = create_test_cache(100, 300);
        let result = create_result(&[100, 200, 300]);

        cache.set("test_key".to_string(), result.clone());

        let cached = cache.get("test_key").expect("Should have cached value");
        assert_eq!(cached, result);
    }

    #[test]
    fn test_cache_miss() {
        let cache = create_test_cache(100, 300);

        let cached = cache.get("nonexistent");
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_len() {
        let cache = create_test_cache(100, 300);

        cache.set("key1".to_string(), create_result(&[1]));
        cache.set("key2".to_string(), create_result(&[2]));
        cache.set("key3".to_string(), create_result(&[3]));

        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_cache_clear() {
        let cache = create_test_cache(100, 300);

        cache.set("key1".to_string(), create_result(&[1]));
        cache.set("key2".to_string(), create_result(&[2]));

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert!(cache.is_empty());
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_cache_contains() {
        let cache = create_test_cache(100, 300);

        cache.set("exists".to_string(), create_result(&[1]));

        assert!(cache.contains("exists"));
        assert!(!cache.contains("not_exists"));
    }

    // LRU eviction tests

    #[test]
    fn test_lru_eviction() {
        // Create a cache with only 3 entries
        let cache = create_test_cache(3, 300);

        // Fill the cache
        cache.set("key1".to_string(), create_result(&[1]));
        cache.set("key2".to_string(), create_result(&[2]));
        cache.set("key3".to_string(), create_result(&[3]));

        assert_eq!(cache.len(), 3);

        // Access key1 to make it recently used
        let _ = cache.get("key1");

        // Add a new entry - should evict key2 (LRU)
        cache.set("key4".to_string(), create_result(&[4]));

        assert_eq!(cache.len(), 3);
        assert!(cache.get("key1").is_some()); // Recently accessed
        assert!(cache.get("key2").is_none()); // Evicted (LRU)
        assert!(cache.get("key3").is_some());
        assert!(cache.get("key4").is_some());
    }

    #[test]
    fn test_lru_order_on_access() {
        let cache = create_test_cache(2, 300);

        cache.set("key1".to_string(), create_result(&[1]));
        cache.set("key2".to_string(), create_result(&[2]));

        // Access key1 to make it MRU
        let _ = cache.get("key1");

        // Add key3 - should evict key2 (now LRU)
        cache.set("key3".to_string(), create_result(&[3]));

        assert!(cache.get("key1").is_some());
        assert!(cache.get("key2").is_none());
        assert!(cache.get("key3").is_some());
    }

    // TTL expiration tests

    #[test]
    fn test_ttl_expiration() {
        // Create cache with 100ms TTL
        let cache = QueryCache::with_capacity(100, Duration::from_millis(100));

        cache.set("expires".to_string(), create_result(&[1]));

        // Should be valid immediately
        assert!(cache.get("expires").is_some());

        // Wait for expiration
        thread::sleep(Duration::from_millis(150));

        // Should be expired now
        assert!(cache.get("expires").is_none());
    }

    #[test]
    fn test_ttl_cleanup() {
        let cache = QueryCache::with_capacity(100, Duration::from_millis(50));

        cache.set("key1".to_string(), create_result(&[1]));
        cache.set("key2".to_string(), create_result(&[2]));

        assert_eq!(cache.len(), 2);

        // Wait for expiration
        thread::sleep(Duration::from_millis(100));

        // Cleanup expired entries
        cache.cleanup_expired();

        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        let cache = QueryCache::with_capacity(100, Duration::from_millis(50));

        cache.set("key1".to_string(), create_result(&[1]));
        cache.set("key2".to_string(), create_result(&[2]));

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.expired_entries, 0);

        // Wait for expiration
        thread::sleep(Duration::from_millis(100));

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.expired_entries, 2);
    }

    // Thread-safety tests

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;

        let cache = Arc::new(create_test_cache(1000, 300));
        let mut handles = vec![];

        // Spawn 10 threads that each write and read 10 entries
        for thread_id in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                for i in 0..10 {
                    let key = format!("thread{}_{}", thread_id, i);
                    let result = create_result(&[(thread_id * 100 + i) as SctId]);
                    cache_clone.set(key.clone(), result.clone());

                    // Read it back
                    let cached = cache_clone.get(&key);
                    assert!(cached.is_some());
                    assert_eq!(cached.unwrap(), result);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Should have 100 entries
        assert_eq!(cache.len(), 100);
    }

    #[test]
    fn test_concurrent_reads() {
        use std::sync::Arc;

        let cache = Arc::new(create_test_cache(100, 300));
        let result = create_result(&[1, 2, 3, 4, 5]);
        cache.set("shared".to_string(), result.clone());

        let mut handles = vec![];

        // Spawn 20 threads that all read the same key
        for _ in 0..20 {
            let cache_clone = Arc::clone(&cache);
            let expected = result.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let cached = cache_clone.get("shared");
                    assert_eq!(cached, Some(expected.clone()));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    // Cache key normalization tests

    #[test]
    fn test_normalize_cache_key() {
        // Basic normalization
        assert_eq!(normalize_cache_key("<<  73211009"), "<< 73211009");
        assert_eq!(normalize_cache_key("  << 73211009  "), "<< 73211009");
        assert_eq!(normalize_cache_key("<< 73211009"), "<< 73211009");
    }

    #[test]
    fn test_normalize_cache_key_whitespace() {
        // Multiple spaces
        assert_eq!(
            normalize_cache_key("< 100   AND   < 200"),
            "< 100 AND < 200"
        );

        // Tabs and newlines
        assert_eq!(normalize_cache_key("<< 100\t"), "<< 100");
        assert_eq!(normalize_cache_key("<< 100\n"), "<< 100");
    }

    #[test]
    fn test_normalize_cache_key_empty() {
        assert_eq!(normalize_cache_key(""), "");
        assert_eq!(normalize_cache_key("   "), "");
    }

    // Configuration tests

    #[test]
    fn test_cache_intermediates_flag() {
        let config = CacheConfig {
            max_entries: 100,
            ttl: Duration::from_secs(300),
            cache_intermediates: true,
        };
        let cache = QueryCache::new(config);
        assert!(cache.should_cache_intermediates());

        let config2 = CacheConfig {
            cache_intermediates: false,
            ..CacheConfig::default()
        };
        let cache2 = QueryCache::new(config2);
        assert!(!cache2.should_cache_intermediates());
    }

    #[test]
    fn test_cache_debug() {
        let cache = create_test_cache(100, 300);
        cache.set("key".to_string(), create_result(&[1]));

        let debug = format!("{:?}", cache);
        assert!(debug.contains("QueryCache"));
        assert!(debug.contains("entries"));
    }

    // Edge cases

    #[test]
    fn test_cache_update_existing_key() {
        let cache = create_test_cache(100, 300);

        cache.set("key".to_string(), create_result(&[1, 2]));
        assert_eq!(cache.get("key"), Some(create_result(&[1, 2])));

        // Update the same key
        cache.set("key".to_string(), create_result(&[3, 4, 5]));
        assert_eq!(cache.get("key"), Some(create_result(&[3, 4, 5])));

        // Should still be 1 entry
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_empty_result() {
        let cache = create_test_cache(100, 300);

        // Cache an empty result (valid use case)
        cache.set("empty".to_string(), HashSet::new());

        let cached = cache.get("empty");
        assert!(cached.is_some());
        assert!(cached.unwrap().is_empty());
    }

    #[test]
    fn test_cache_min_capacity() {
        // Capacity of 0 should be treated as 1
        let cache = create_test_cache(0, 300);

        cache.set("key1".to_string(), create_result(&[1]));
        cache.set("key2".to_string(), create_result(&[2]));

        // Should only have 1 entry (key2)
        assert_eq!(cache.len(), 1);
        assert!(cache.get("key2").is_some());
    }
}
