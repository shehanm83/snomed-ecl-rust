//! Query result types for ECL execution.

use std::collections::HashSet;
use std::time::Duration;

use snomed_ecl::SctId;

/// Result of an ECL query execution.
///
/// Contains the matching concept IDs and execution statistics.
///
/// # Example
///
/// ```ignore
/// let result = executor.execute("<< 73211009")?;
///
/// println!("Found {} concepts", result.count());
///
/// if result.contains(46635009) {
///     println!("Type 2 diabetes is included");
/// }
///
/// for concept_id in result.iter() {
///     println!("Concept: {}", concept_id);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Set of matching concept IDs.
    pub concept_ids: HashSet<SctId>,
    /// Execution statistics.
    pub stats: ExecutionStats,
}

impl QueryResult {
    /// Creates a new QueryResult with the given concept IDs.
    pub fn new(concept_ids: HashSet<SctId>, stats: ExecutionStats) -> Self {
        Self { concept_ids, stats }
    }

    /// Creates an empty QueryResult.
    pub fn empty() -> Self {
        Self {
            concept_ids: HashSet::new(),
            stats: ExecutionStats::default(),
        }
    }

    /// Returns the number of matching concepts.
    pub fn count(&self) -> usize {
        self.concept_ids.len()
    }

    /// Returns true if the result set is empty.
    pub fn is_empty(&self) -> bool {
        self.concept_ids.is_empty()
    }

    /// Checks if a specific concept is in the result set.
    pub fn contains(&self, concept_id: SctId) -> bool {
        self.concept_ids.contains(&concept_id)
    }

    /// Returns an iterator over matching concept IDs.
    pub fn iter(&self) -> impl Iterator<Item = &SctId> {
        self.concept_ids.iter()
    }

    /// Converts the result set to a sorted Vec.
    pub fn to_vec(&self) -> Vec<SctId> {
        let mut vec: Vec<SctId> = self.concept_ids.iter().copied().collect();
        vec.sort_unstable();
        vec
    }
}

impl IntoIterator for QueryResult {
    type Item = SctId;
    type IntoIter = std::collections::hash_set::IntoIter<SctId>;

    fn into_iter(self) -> Self::IntoIter {
        self.concept_ids.into_iter()
    }
}

impl<'a> IntoIterator for &'a QueryResult {
    type Item = &'a SctId;
    type IntoIter = std::collections::hash_set::Iter<'a, SctId>;

    fn into_iter(self) -> Self::IntoIter {
        self.concept_ids.iter()
    }
}

/// Statistics from ECL query execution.
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Total execution duration.
    pub duration: Duration,
    /// Number of concepts traversed during execution.
    pub concepts_traversed: usize,
    /// Whether the result was served from cache.
    pub cache_hit: bool,
}

impl ExecutionStats {
    /// Creates new execution stats.
    pub fn new(duration: Duration, concepts_traversed: usize, cache_hit: bool) -> Self {
        Self {
            duration,
            concepts_traversed,
            cache_hit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_result_empty() {
        let result = QueryResult::empty();
        assert_eq!(result.count(), 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_query_result_with_concepts() {
        let concept_ids: HashSet<SctId> = [100, 200, 300].into_iter().collect();
        let stats = ExecutionStats::default();
        let result = QueryResult::new(concept_ids, stats);

        assert_eq!(result.count(), 3);
        assert!(!result.is_empty());
        assert!(result.contains(100));
        assert!(result.contains(200));
        assert!(result.contains(300));
        assert!(!result.contains(400));
    }

    #[test]
    fn test_query_result_iter() {
        let concept_ids: HashSet<SctId> = [100, 200].into_iter().collect();
        let result = QueryResult::new(concept_ids, ExecutionStats::default());

        let ids: HashSet<&SctId> = result.iter().collect();
        assert!(ids.contains(&100));
        assert!(ids.contains(&200));
    }

    #[test]
    fn test_query_result_to_vec() {
        let concept_ids: HashSet<SctId> = [300, 100, 200].into_iter().collect();
        let result = QueryResult::new(concept_ids, ExecutionStats::default());

        let vec = result.to_vec();
        assert_eq!(vec, vec![100, 200, 300]);
    }

    #[test]
    fn test_query_result_into_iter() {
        let concept_ids: HashSet<SctId> = [100, 200].into_iter().collect();
        let result = QueryResult::new(concept_ids, ExecutionStats::default());

        let collected: HashSet<SctId> = result.into_iter().collect();
        assert!(collected.contains(&100));
        assert!(collected.contains(&200));
    }

    #[test]
    fn test_execution_stats() {
        let stats = ExecutionStats::new(Duration::from_millis(50), 1000, true);

        assert_eq!(stats.duration, Duration::from_millis(50));
        assert_eq!(stats.concepts_traversed, 1000);
        assert!(stats.cache_hit);
    }
}
