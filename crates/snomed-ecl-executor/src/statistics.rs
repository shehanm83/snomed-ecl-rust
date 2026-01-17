//! Hierarchy statistics for ECL query planning.
//!
//! Provides statistics about SNOMED CT concept hierarchies for estimating
//! query cardinality and generating optimization hints.

use std::collections::HashMap;

use snomed_types::SctId;

use crate::traits::EclQueryable;

/// Well-known SNOMED CT concept IDs with pre-computed statistics.
pub mod well_known {
    use snomed_types::SctId;

    /// Clinical finding (finding) - ~400K descendants
    pub const CLINICAL_FINDING: SctId = 404684003;

    /// Body structure - ~50K descendants
    pub const BODY_STRUCTURE: SctId = 123037004;

    /// Procedure - ~100K descendants
    pub const PROCEDURE: SctId = 71388002;

    /// Substance - ~50K descendants
    pub const SUBSTANCE: SctId = 105590001;

    /// Pharmaceutical/biologic product - ~40K descendants
    pub const PRODUCT: SctId = 373873005;

    /// Qualifier value - ~15K descendants
    pub const QUALIFIER_VALUE: SctId = 362981000;

    /// Observable entity - ~20K descendants
    pub const OBSERVABLE_ENTITY: SctId = 363787002;

    /// Event - ~5K descendants
    pub const EVENT: SctId = 272379006;

    /// SNOMED CT root concept
    pub const ROOT_CONCEPT: SctId = 138875005;
}

/// Default heuristics for cardinality estimation.
pub mod heuristics {
    /// Average children per concept in SNOMED CT
    pub const AVG_CHILDREN_PER_CONCEPT: usize = 5;

    /// Average hierarchy depth in SNOMED CT
    pub const AVG_HIERARCHY_DEPTH: usize = 15;

    /// Default estimate when concept statistics are unknown
    pub const DEFAULT_DESCENDANT_ESTIMATE: usize = 100;

    /// Default estimate for unknown ancestor count
    pub const DEFAULT_ANCESTOR_ESTIMATE: usize = 10;

    /// Selectivity factor for AND operations (typical overlap)
    pub const AND_SELECTIVITY_FACTOR: f64 = 0.3;

    /// Overlap factor for MINUS operations (typical overlap)
    pub const MINUS_OVERLAP_FACTOR: f64 = 0.1;

    /// Threshold for considering a traversal "large"
    pub const LARGE_TRAVERSAL_THRESHOLD: usize = 100_000;
}

/// Cost model constants for query planning.
pub mod cost {
    /// Cost per concept lookup (relative units, ~0.001ms)
    pub const CONCEPT_LOOKUP: f64 = 0.001;

    /// Cost per single level traversal per concept (relative units, ~0.01ms)
    pub const SINGLE_LEVEL_TRAVERSAL: f64 = 0.01;

    /// Cost per concept in full descendant traversal (relative units, ~0.001ms)
    pub const DESCENDANT_TRAVERSAL: f64 = 0.001;

    /// Cost per element in set intersection (relative units, ~0.0001ms)
    pub const SET_INTERSECTION: f64 = 0.0001;

    /// Cost per element in set union (relative units, ~0.00005ms)
    pub const SET_UNION: f64 = 0.00005;

    /// Cost per element in set difference (relative units, ~0.0001ms)
    pub const SET_DIFFERENCE: f64 = 0.0001;
}

/// Statistics service for estimating query cardinality.
///
/// Provides pre-computed and heuristic-based statistics for estimating
/// the cardinality of ECL query results.
#[derive(Debug)]
pub struct StatisticsService {
    /// Pre-computed descendant counts for well-known concepts.
    well_known_counts: HashMap<SctId, usize>,
    /// Cached concept child counts.
    child_counts: HashMap<SctId, usize>,
}

impl Default for StatisticsService {
    fn default() -> Self {
        Self::new()
    }
}

impl StatisticsService {
    /// Creates a new statistics service with default well-known concept counts.
    pub fn new() -> Self {
        let mut well_known_counts = HashMap::new();

        // Pre-computed estimates for top-level hierarchies
        well_known_counts.insert(well_known::CLINICAL_FINDING, 400_000);
        well_known_counts.insert(well_known::BODY_STRUCTURE, 50_000);
        well_known_counts.insert(well_known::PROCEDURE, 100_000);
        well_known_counts.insert(well_known::SUBSTANCE, 50_000);
        well_known_counts.insert(well_known::PRODUCT, 40_000);
        well_known_counts.insert(well_known::QUALIFIER_VALUE, 15_000);
        well_known_counts.insert(well_known::OBSERVABLE_ENTITY, 20_000);
        well_known_counts.insert(well_known::EVENT, 5_000);
        well_known_counts.insert(well_known::ROOT_CONCEPT, 500_000);

        Self {
            well_known_counts,
            child_counts: HashMap::new(),
        }
    }

    /// Creates a statistics service with custom concept counts.
    ///
    /// Use this to provide actual statistics from a loaded SNOMED store.
    pub fn with_counts(concept_counts: HashMap<SctId, usize>) -> Self {
        Self {
            well_known_counts: concept_counts,
            child_counts: HashMap::new(),
        }
    }

    /// Populates child counts from a queryable store.
    ///
    /// This can be called lazily to cache actual child counts for
    /// concepts that are queried frequently.
    pub fn populate_from_store(&mut self, store: &dyn EclQueryable, concept_id: SctId) {
        self.child_counts
            .entry(concept_id)
            .or_insert_with(|| store.get_children(concept_id).len());
    }

    /// Estimates the number of descendants for a concept.
    ///
    /// Uses pre-computed statistics for well-known concepts,
    /// otherwise falls back to heuristics.
    pub fn estimated_descendants(&self, concept_id: SctId) -> usize {
        // Check well-known counts first
        if let Some(&count) = self.well_known_counts.get(&concept_id) {
            return count;
        }

        // Check cached child counts for heuristic estimate
        if let Some(&child_count) = self.child_counts.get(&concept_id) {
            // Estimate descendants based on child count and average depth
            // descendants ≈ children * avg_children^(avg_depth - 1)
            if child_count == 0 {
                return 0;
            }
            // Simple heuristic: children * 10 (approximate depth expansion)
            return child_count * 10;
        }

        // Fall back to default
        heuristics::DEFAULT_DESCENDANT_ESTIMATE
    }

    /// Estimates the number of ancestors for a concept.
    ///
    /// Ancestors are typically much fewer than descendants.
    pub fn estimated_ancestors(&self, _concept_id: SctId) -> usize {
        // Ancestors are typically bounded by hierarchy depth
        heuristics::AVG_HIERARCHY_DEPTH
    }

    /// Estimates cardinality for a self constraint (single concept).
    pub fn estimated_self(&self, _concept_id: SctId) -> usize {
        1
    }

    /// Estimates cardinality for direct children.
    pub fn estimated_children(&self, concept_id: SctId) -> usize {
        if let Some(&count) = self.child_counts.get(&concept_id) {
            return count;
        }
        heuristics::AVG_CHILDREN_PER_CONCEPT
    }

    /// Estimates cardinality for direct parents.
    pub fn estimated_parents(&self, _concept_id: SctId) -> usize {
        // Most concepts have 1-3 parents in SNOMED CT
        2
    }

    /// Estimates cardinality for AND (intersection) of two sets.
    pub fn estimated_and(&self, left: usize, right: usize) -> usize {
        let smaller = left.min(right);
        let selectivity = heuristics::AND_SELECTIVITY_FACTOR;
        ((smaller as f64) * selectivity).ceil() as usize
    }

    /// Estimates cardinality for OR (union) of two sets.
    pub fn estimated_or(&self, left: usize, right: usize) -> usize {
        // Union is at most sum minus overlap
        let overlap = self.estimated_and(left, right);
        left + right - overlap
    }

    /// Estimates cardinality for MINUS (difference) of two sets.
    pub fn estimated_minus(&self, left: usize, right: usize) -> usize {
        // Estimate overlap and subtract
        let overlap_factor = heuristics::MINUS_OVERLAP_FACTOR;
        let overlap = ((left.min(right) as f64) * overlap_factor).ceil() as usize;
        left.saturating_sub(overlap)
    }

    /// Estimates execution cost for a descendant traversal.
    pub fn cost_descendants(&self, estimated_count: usize) -> f64 {
        (estimated_count as f64) * cost::DESCENDANT_TRAVERSAL
    }

    /// Estimates execution cost for an ancestor traversal.
    pub fn cost_ancestors(&self, estimated_count: usize) -> f64 {
        // Ancestor traversal is typically upward, fewer nodes
        (estimated_count as f64) * cost::SINGLE_LEVEL_TRAVERSAL
    }

    /// Estimates execution cost for a concept lookup.
    pub fn cost_lookup(&self) -> f64 {
        cost::CONCEPT_LOOKUP
    }

    /// Estimates execution cost for set intersection.
    pub fn cost_intersection(&self, set_size: usize) -> f64 {
        (set_size as f64) * cost::SET_INTERSECTION
    }

    /// Estimates execution cost for set union.
    pub fn cost_union(&self, set_size: usize) -> f64 {
        (set_size as f64) * cost::SET_UNION
    }

    /// Estimates execution cost for set difference.
    pub fn cost_difference(&self, set_size: usize) -> f64 {
        (set_size as f64) * cost::SET_DIFFERENCE
    }

    /// Checks if a traversal would be considered "large" (expensive).
    pub fn is_large_traversal(&self, estimated_count: usize) -> bool {
        estimated_count > heuristics::LARGE_TRAVERSAL_THRESHOLD
    }

    /// Registers a known descendant count for a concept.
    ///
    /// Use this to provide actual statistics after executing queries.
    pub fn register_descendant_count(&mut self, concept_id: SctId, count: usize) {
        self.well_known_counts.insert(concept_id, count);
    }

    /// Registers a child count for a concept.
    pub fn register_child_count(&mut self, concept_id: SctId, count: usize) {
        self.child_counts.insert(concept_id, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_service_new() {
        let stats = StatisticsService::new();
        assert!(stats
            .well_known_counts
            .contains_key(&well_known::CLINICAL_FINDING));
    }

    #[test]
    fn test_estimated_descendants_well_known() {
        let stats = StatisticsService::new();

        // Well-known concept should return pre-computed value
        let count = stats.estimated_descendants(well_known::CLINICAL_FINDING);
        assert_eq!(count, 400_000);

        let count = stats.estimated_descendants(well_known::BODY_STRUCTURE);
        assert_eq!(count, 50_000);
    }

    #[test]
    fn test_estimated_descendants_unknown() {
        let stats = StatisticsService::new();

        // Unknown concept should return default
        let count = stats.estimated_descendants(12345);
        assert_eq!(count, heuristics::DEFAULT_DESCENDANT_ESTIMATE);
    }

    #[test]
    fn test_estimated_self() {
        let stats = StatisticsService::new();
        assert_eq!(stats.estimated_self(12345), 1);
    }

    #[test]
    fn test_estimated_ancestors() {
        let stats = StatisticsService::new();
        assert_eq!(
            stats.estimated_ancestors(12345),
            heuristics::AVG_HIERARCHY_DEPTH
        );
    }

    #[test]
    fn test_estimated_and() {
        let stats = StatisticsService::new();

        // AND with selectivity
        let result = stats.estimated_and(1000, 500);
        assert!(result <= 500); // Should be <= smaller operand
        assert!(result > 0);
    }

    #[test]
    fn test_estimated_or() {
        let stats = StatisticsService::new();

        // OR should be between max and sum
        let result = stats.estimated_or(1000, 500);
        assert!(result >= 1000); // At least the larger
        assert!(result <= 1500); // At most the sum
    }

    #[test]
    fn test_estimated_minus() {
        let stats = StatisticsService::new();

        // MINUS should be less than left
        let result = stats.estimated_minus(1000, 500);
        assert!(result <= 1000);
        assert!(result > 0);
    }

    #[test]
    fn test_cost_calculations() {
        let stats = StatisticsService::new();

        let desc_cost = stats.cost_descendants(1000);
        assert!(desc_cost > 0.0);

        let anc_cost = stats.cost_ancestors(100);
        assert!(anc_cost > 0.0);

        let lookup_cost = stats.cost_lookup();
        assert!(lookup_cost > 0.0);
    }

    #[test]
    fn test_is_large_traversal() {
        let stats = StatisticsService::new();

        assert!(!stats.is_large_traversal(50_000));
        assert!(stats.is_large_traversal(150_000));
    }

    #[test]
    fn test_register_counts() {
        let mut stats = StatisticsService::new();

        // Register custom count
        stats.register_descendant_count(99999, 5000);
        assert_eq!(stats.estimated_descendants(99999), 5000);

        stats.register_child_count(88888, 25);
        assert_eq!(stats.estimated_children(88888), 25);
    }

    #[test]
    fn test_with_custom_counts() {
        let mut counts = HashMap::new();
        counts.insert(11111, 1000);
        counts.insert(22222, 2000);

        let stats = StatisticsService::with_counts(counts);

        assert_eq!(stats.estimated_descendants(11111), 1000);
        assert_eq!(stats.estimated_descendants(22222), 2000);
        // Unknown concept falls back to default
        assert_eq!(
            stats.estimated_descendants(33333),
            heuristics::DEFAULT_DESCENDANT_ESTIMATE
        );
    }

    #[test]
    fn test_estimated_children_with_cache() {
        let mut stats = StatisticsService::new();

        // Without cache, returns default
        assert_eq!(
            stats.estimated_children(12345),
            heuristics::AVG_CHILDREN_PER_CONCEPT
        );

        // Register child count
        stats.register_child_count(12345, 10);
        assert_eq!(stats.estimated_children(12345), 10);
    }
}
