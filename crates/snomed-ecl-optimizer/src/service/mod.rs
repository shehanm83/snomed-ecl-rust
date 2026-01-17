//! Runtime ECL filtering service with caching.
//!
//! This module provides a high-level service for filtering concepts
//! by ECL constraints with built-in caching for repeated queries.
//!
//! # Example
//!
//! ```ignore
//! use snomed_ecl_optimizer::service::EclFilterService;
//!
//! // Create the service
//! let service = EclFilterService::new(&store);
//!
//! // Filter candidates by ECL constraint
//! let result = service.filter(&candidates, "<< 404684003")?;
//! println!("Passed: {} / {}", result.filtered_count, result.original_count);
//!
//! // Check single concept
//! if service.matches(concept_id, "<< 73211009")? {
//!     println!("Concept is a type of diabetes");
//! }
//!
//! // Warm cache with common expressions
//! service.warm_cache(&[
//!     "<< 404684003", // Clinical finding
//!     "<< 123037004", // Body structure
//! ]);
//! ```

mod types;

pub use types::{FilterResult, FilterStats};

use lru::LruCache;
use parking_lot::RwLock;
use snomed_ecl::SctId;
use snomed_ecl_executor::{EclExecutor, EclQueryable, ExecutorConfig};
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::time::Instant;

use crate::error::OptimizerResult;

/// Configuration for the filter service.
#[derive(Debug, Clone)]
pub struct FilterServiceConfig {
    /// Maximum number of cached query results.
    pub cache_size: usize,
    /// Whether to cache results at all.
    pub enable_cache: bool,
}

impl Default for FilterServiceConfig {
    fn default() -> Self {
        Self {
            cache_size: 10_000,
            enable_cache: true,
        }
    }
}

impl FilterServiceConfig {
    /// Creates a config with no caching.
    pub fn no_cache() -> Self {
        Self {
            cache_size: 0,
            enable_cache: false,
        }
    }

    /// Creates a config with custom cache size.
    pub fn with_cache_size(size: usize) -> Self {
        Self {
            cache_size: size,
            enable_cache: size > 0,
        }
    }
}

/// A high-level service for filtering concepts by ECL constraints.
///
/// Provides caching and convenience methods for common operations.
pub struct EclFilterService<'a, T: EclQueryable> {
    executor: EclExecutor<'a>,
    #[allow(dead_code)]
    store: &'a T,
    config: FilterServiceConfig,
    cache: Option<RwLock<LruCache<String, HashSet<SctId>>>>,
    stats: RwLock<FilterStats>,
}

impl<'a, T: EclQueryable> EclFilterService<'a, T> {
    /// Creates a new filter service with default configuration.
    pub fn new(store: &'a T) -> Self {
        Self::with_config(store, FilterServiceConfig::default())
    }

    /// Creates a new filter service with custom configuration.
    pub fn with_config(store: &'a T, config: FilterServiceConfig) -> Self {
        let executor = EclExecutor::with_config(store, ExecutorConfig::default());

        let cache = if config.enable_cache && config.cache_size > 0 {
            let size = NonZeroUsize::new(config.cache_size).unwrap();
            Some(RwLock::new(LruCache::new(size)))
        } else {
            None
        };

        Self {
            executor,
            store,
            config,
            cache,
            stats: RwLock::new(FilterStats::default()),
        }
    }

    /// Filters a set of candidate concept IDs by an ECL expression.
    ///
    /// Returns only the concepts that match the expression.
    pub fn filter(&self, candidates: &[SctId], ecl: &str) -> OptimizerResult<FilterResult> {
        let start = Instant::now();
        let original_count = candidates.len();

        // Get matching concepts (from cache or execute)
        let matching = self.get_matching_concepts(ecl)?;

        // Filter candidates
        let filtered_ids: Vec<SctId> = candidates
            .iter()
            .copied()
            .filter(|id| matching.contains(id))
            .collect();

        let filter_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.queries_executed += 1;
            stats.total_filter_time_ms += filter_time_ms;
            stats.total_candidates_filtered += original_count;
        }

        Ok(FilterResult {
            filtered_ids,
            original_count,
            filter_time_ms,
        })
    }

    /// Checks if a concept matches an ECL expression.
    pub fn matches(&self, concept_id: SctId, ecl: &str) -> OptimizerResult<bool> {
        let matching = self.get_matching_concepts(ecl)?;
        Ok(matching.contains(&concept_id))
    }

    /// Executes an ECL query and returns all matching concepts.
    pub fn execute(&self, ecl: &str) -> OptimizerResult<HashSet<SctId>> {
        self.get_matching_concepts(ecl)
    }

    /// Gets matching concepts, using cache if available.
    fn get_matching_concepts(&self, ecl: &str) -> OptimizerResult<HashSet<SctId>> {
        let cache_key = normalize_ecl(ecl);

        // Check cache first
        if let Some(ref cache) = self.cache {
            let cache_read = cache.read();
            if let Some(cached) = cache_read.peek(&cache_key) {
                let mut stats = self.stats.write();
                stats.cache_hits += 1;
                return Ok(cached.clone());
            }
            drop(cache_read);
        }

        // Execute the query
        let result = self.executor.execute(ecl)?;
        let concepts: HashSet<SctId> = result.concept_ids.clone();

        // Update cache
        if let Some(ref cache) = self.cache {
            let mut cache_write = cache.write();
            cache_write.put(cache_key, concepts.clone());
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.cache_misses += 1;
        }

        Ok(concepts)
    }

    /// Warms the cache with the given ECL expressions.
    ///
    /// This pre-executes queries so subsequent calls are cache hits.
    pub fn warm_cache(&self, expressions: &[&str]) {
        for ecl in expressions {
            let _ = self.get_matching_concepts(ecl);
        }
    }

    /// Clears the cache.
    pub fn clear_cache(&self) {
        if let Some(ref cache) = self.cache {
            cache.write().clear();
        }
    }

    /// Returns the current cache size.
    pub fn cache_len(&self) -> usize {
        self.cache.as_ref().map(|c| c.read().len()).unwrap_or(0)
    }

    /// Returns filter statistics.
    pub fn stats(&self) -> FilterStats {
        self.stats.read().clone()
    }

    /// Resets statistics.
    pub fn reset_stats(&self) {
        *self.stats.write() = FilterStats::default();
    }

    /// Returns a reference to the underlying executor.
    pub fn executor(&self) -> &EclExecutor<'a> {
        &self.executor
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &FilterServiceConfig {
        &self.config
    }
}

/// Normalizes an ECL expression for cache key consistency.
fn normalize_ecl(ecl: &str) -> String {
    ecl.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Mock store for testing.
    struct MockStore {
        concepts: HashSet<SctId>,
        children: HashMap<SctId, Vec<SctId>>,
        parents: HashMap<SctId, Vec<SctId>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                concepts: HashSet::new(),
                children: HashMap::new(),
                parents: HashMap::new(),
            }
        }

        fn add_concept(&mut self, id: SctId) {
            self.concepts.insert(id);
        }

        fn add_is_a(&mut self, child: SctId, parent: SctId) {
            self.children.entry(parent).or_default().push(child);
            self.parents.entry(child).or_default().push(parent);
        }
    }

    impl EclQueryable for MockStore {
        fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
            self.children.get(&concept_id).cloned().unwrap_or_default()
        }

        fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
            self.parents.get(&concept_id).cloned().unwrap_or_default()
        }

        fn has_concept(&self, concept_id: SctId) -> bool {
            self.concepts.contains(&concept_id)
        }

        fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> {
            Box::new(self.concepts.iter().copied())
        }

        fn get_refset_members(&self, _refset_id: SctId) -> Vec<SctId> {
            Vec::new()
        }
    }

    fn create_test_store() -> MockStore {
        let mut store = MockStore::new();

        for id in [100, 200, 300, 400, 500, 600] {
            store.add_concept(id);
        }

        store.add_is_a(200, 100);
        store.add_is_a(300, 100);
        store.add_is_a(400, 200);
        store.add_is_a(500, 200);
        store.add_is_a(600, 300);

        store
    }

    #[test]
    fn test_filter() {
        let store = create_test_store();
        let service = EclFilterService::new(&store);

        // Filter candidates by descendants of 100
        let candidates = vec![200, 300, 400, 999];
        let result = service.filter(&candidates, "<< 100").unwrap();

        // 200, 300, 400 are descendants of 100, 999 is not a valid concept
        assert_eq!(result.original_count, 4);
        assert_eq!(result.filtered_count(), 3);
        assert!(result.filtered_ids.contains(&200));
        assert!(result.filtered_ids.contains(&300));
        assert!(result.filtered_ids.contains(&400));
    }

    #[test]
    fn test_matches() {
        let store = create_test_store();
        let service = EclFilterService::new(&store);

        // 400 is a descendant of 100
        assert!(service.matches(400, "<< 100").unwrap());

        // 100 is not a descendant of 200
        assert!(!service.matches(100, "<< 200").unwrap());
    }

    #[test]
    fn test_cache() {
        let store = create_test_store();
        let service = EclFilterService::new(&store);

        // First call - cache miss
        let _ = service.execute("<< 100").unwrap();
        assert_eq!(service.stats().cache_misses, 1);
        assert_eq!(service.stats().cache_hits, 0);

        // Second call - cache hit
        let _ = service.execute("<< 100").unwrap();
        assert_eq!(service.stats().cache_misses, 1);
        assert_eq!(service.stats().cache_hits, 1);

        assert_eq!(service.cache_len(), 1);
    }

    #[test]
    fn test_warm_cache() {
        let store = create_test_store();
        let service = EclFilterService::new(&store);

        service.warm_cache(&["<< 100", "<< 200"]);

        assert_eq!(service.cache_len(), 2);
    }

    #[test]
    fn test_clear_cache() {
        let store = create_test_store();
        let service = EclFilterService::new(&store);

        service.warm_cache(&["<< 100", "<< 200"]);
        assert_eq!(service.cache_len(), 2);

        service.clear_cache();
        assert_eq!(service.cache_len(), 0);
    }

    #[test]
    fn test_no_cache_config() {
        let store = create_test_store();
        let service = EclFilterService::with_config(&store, FilterServiceConfig::no_cache());

        // Execute twice
        let _ = service.execute("<< 100").unwrap();
        let _ = service.execute("<< 100").unwrap();

        // No cache, so both are misses
        assert_eq!(service.stats().cache_hits, 0);
        assert_eq!(service.cache_len(), 0);
    }

    #[test]
    fn test_normalize_ecl() {
        assert_eq!(normalize_ecl("<<  100"), "<< 100");
        assert_eq!(normalize_ecl("<< 100   AND  << 200"), "<< 100 AND << 200");
    }
}
