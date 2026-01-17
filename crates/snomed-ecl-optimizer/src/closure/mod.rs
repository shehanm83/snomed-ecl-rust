//! Precomputed transitive closure for O(1) hierarchy lookups.
//!
//! The transitive closure precomputes all ancestor/descendant relationships,
//! trading memory for speed. After building, hierarchy queries are O(1).
//!
//! # Example
//!
//! ```ignore
//! use snomed_ecl_optimizer::closure::TransitiveClosure;
//! use snomed_ecl_executor::{EclExecutor, EclQueryable};
//!
//! // Build closure from your existing store (one-time operation)
//! let closure = TransitiveClosure::build(&my_store);
//!
//! // Check ancestry in O(1)
//! if closure.is_ancestor_of(clinical_finding, diabetes) {
//!     println!("Diabetes is a clinical finding");
//! }
//!
//! // Get all descendants in O(1) - returns reference to pre-built set
//! let descendants = closure.get_descendants(diabetes);
//!
//! // Use directly with EclExecutor (closure implements EclQueryable)
//! let executor = EclExecutor::new(&closure);
//! let result = executor.execute("<< 73211009")?;
//! ```

mod stats;

pub use stats::ClosureStats;

use snomed_ecl::SctId;
use snomed_ecl_executor::EclQueryable;
use std::collections::{HashMap, HashSet, VecDeque};

/// Precomputed transitive closure of the IS-A hierarchy.
///
/// Provides O(1) lookup for:
/// - Is `A` an ancestor of `B`?
/// - Is `A` a descendant of `B`?
/// - Get all ancestors of `A`
/// - Get all descendants of `A`
///
/// This implements [`EclQueryable`], so it can be used directly with
/// [`EclExecutor`](snomed_ecl_executor::EclExecutor).
pub struct TransitiveClosure {
    /// For each concept, the set of all ancestors (transitive IS-A closure).
    ancestors: HashMap<SctId, HashSet<SctId>>,
    /// For each concept, the set of all descendants (inverse transitive closure).
    descendants: HashMap<SctId, HashSet<SctId>>,
    /// Direct parents for each concept.
    parents: HashMap<SctId, Vec<SctId>>,
    /// Direct children for each concept.
    children: HashMap<SctId, Vec<SctId>>,
    /// All known concept IDs.
    concepts: HashSet<SctId>,
    /// Build statistics.
    stats: ClosureStats,
}

impl TransitiveClosure {
    /// Builds the transitive closure from a queryable store.
    ///
    /// This is a one-time O(n * d) operation where n is concept count
    /// and d is the average hierarchy depth.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let closure = TransitiveClosure::build(&my_store);
    /// ```
    pub fn build<T: EclQueryable>(store: &T) -> Self {
        Self::build_with_progress(store, |_, _| {})
    }

    /// Builds the closure with a progress callback.
    ///
    /// The callback receives (current_concept_index, total_concepts).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let closure = TransitiveClosure::build_with_progress(&store, |current, total| {
    ///     println!("Progress: {}/{}", current, total);
    /// });
    /// ```
    pub fn build_with_progress<T, F>(store: &T, mut progress: F) -> Self
    where
        T: EclQueryable,
        F: FnMut(usize, usize),
    {
        let start = std::time::Instant::now();

        // Collect all concept IDs
        let concepts: HashSet<SctId> = store.all_concept_ids().collect();
        let concept_count = concepts.len();

        // Build direct parent/child maps
        let mut parents: HashMap<SctId, Vec<SctId>> = HashMap::with_capacity(concept_count);
        let mut children: HashMap<SctId, Vec<SctId>> = HashMap::with_capacity(concept_count);
        let mut relationship_count = 0;

        for &concept_id in &concepts {
            let concept_parents = store.get_parents(concept_id);
            relationship_count += concept_parents.len();

            for &parent_id in &concept_parents {
                children.entry(parent_id).or_default().push(concept_id);
            }

            if !concept_parents.is_empty() {
                parents.insert(concept_id, concept_parents);
            }
        }

        // Build transitive ancestors for each concept
        let mut ancestors: HashMap<SctId, HashSet<SctId>> = HashMap::with_capacity(concept_count);
        let mut max_depth = 0;
        let mut total_ancestors = 0usize;

        for (idx, &concept_id) in concepts.iter().enumerate() {
            progress(idx, concept_count);

            let (concept_ancestors, depth) = Self::compute_ancestors(concept_id, &parents);
            total_ancestors += concept_ancestors.len();
            max_depth = max_depth.max(depth);

            if !concept_ancestors.is_empty() {
                ancestors.insert(concept_id, concept_ancestors);
            }
        }

        // Build transitive descendants for each concept
        let mut descendants: HashMap<SctId, HashSet<SctId>> = HashMap::with_capacity(concept_count);
        let mut total_descendants = 0usize;

        for &concept_id in &concepts {
            let concept_descendants = Self::compute_descendants(concept_id, &children);
            total_descendants += concept_descendants.len();

            if !concept_descendants.is_empty() {
                descendants.insert(concept_id, concept_descendants);
            }
        }

        let build_time = start.elapsed();

        let stats = ClosureStats {
            concept_count,
            relationship_count,
            max_hierarchy_depth: max_depth,
            avg_ancestors: if concept_count > 0 {
                total_ancestors as f64 / concept_count as f64
            } else {
                0.0
            },
            avg_descendants: if concept_count > 0 {
                total_descendants as f64 / concept_count as f64
            } else {
                0.0
            },
            build_time_ms: build_time.as_millis() as u64,
            memory_estimate_bytes: Self::estimate_memory(
                concept_count,
                total_ancestors,
                total_descendants,
            ),
        };

        Self {
            ancestors,
            descendants,
            parents,
            children,
            concepts,
            stats,
        }
    }

    /// Computes all ancestors of a concept using BFS.
    fn compute_ancestors(
        concept_id: SctId,
        parents: &HashMap<SctId, Vec<SctId>>,
    ) -> (HashSet<SctId>, usize) {
        let mut result = HashSet::new();
        let mut queue = VecDeque::new();
        let mut depth = 0;
        let mut current_level_size;
        let mut next_level_size = 0;

        // Start with direct parents
        if let Some(direct_parents) = parents.get(&concept_id) {
            for &parent_id in direct_parents {
                if result.insert(parent_id) {
                    queue.push_back(parent_id);
                    next_level_size += 1;
                }
            }
        }

        current_level_size = next_level_size;
        next_level_size = 0;

        // BFS to find all ancestors
        while let Some(current) = queue.pop_front() {
            current_level_size -= 1;

            if let Some(current_parents) = parents.get(&current) {
                for &parent_id in current_parents {
                    if result.insert(parent_id) {
                        queue.push_back(parent_id);
                        next_level_size += 1;
                    }
                }
            }

            if current_level_size == 0 && next_level_size > 0 {
                depth += 1;
                current_level_size = next_level_size;
                next_level_size = 0;
            }
        }

        (result, depth)
    }

    /// Computes all descendants of a concept using BFS.
    fn compute_descendants(
        concept_id: SctId,
        children: &HashMap<SctId, Vec<SctId>>,
    ) -> HashSet<SctId> {
        let mut result = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct children
        if let Some(direct_children) = children.get(&concept_id) {
            for &child_id in direct_children {
                if result.insert(child_id) {
                    queue.push_back(child_id);
                }
            }
        }

        // BFS to find all descendants
        while let Some(current) = queue.pop_front() {
            if let Some(current_children) = children.get(&current) {
                for &child_id in current_children {
                    if result.insert(child_id) {
                        queue.push_back(child_id);
                    }
                }
            }
        }

        result
    }

    /// Estimates memory usage in bytes.
    fn estimate_memory(
        concept_count: usize,
        total_ancestors: usize,
        total_descendants: usize,
    ) -> usize {
        // Rough estimate: HashMap overhead + HashSet entries (8 bytes each for SctId)
        let hashmap_overhead = concept_count * 48; // HashMap entry overhead
        let ancestor_storage = total_ancestors * 8;
        let descendant_storage = total_descendants * 8;
        let parent_child_storage = concept_count * 16; // Vec overhead per entry

        hashmap_overhead + ancestor_storage + descendant_storage + parent_child_storage
    }

    /// Returns true if `ancestor` is an ancestor of `descendant` (O(1)).
    #[inline]
    pub fn is_ancestor_of(&self, ancestor: SctId, descendant: SctId) -> bool {
        self.ancestors
            .get(&descendant)
            .is_some_and(|anc| anc.contains(&ancestor))
    }

    /// Returns true if `descendant` is a descendant of `ancestor` (O(1)).
    #[inline]
    pub fn is_descendant_of(&self, descendant: SctId, ancestor: SctId) -> bool {
        self.descendants
            .get(&ancestor)
            .is_some_and(|desc| desc.contains(&descendant))
    }

    /// Gets all ancestors of a concept (O(1) - returns reference).
    ///
    /// Returns `None` if the concept has no ancestors (root concept).
    #[inline]
    pub fn get_ancestors(&self, concept_id: SctId) -> Option<&HashSet<SctId>> {
        self.ancestors.get(&concept_id)
    }

    /// Gets all ancestors of a concept, including self.
    pub fn get_ancestors_or_self(&self, concept_id: SctId) -> HashSet<SctId> {
        let mut result = self
            .ancestors
            .get(&concept_id)
            .cloned()
            .unwrap_or_default();
        result.insert(concept_id);
        result
    }

    /// Gets all descendants of a concept (O(1) - returns reference).
    ///
    /// Returns `None` if the concept has no descendants (leaf concept).
    #[inline]
    pub fn get_descendants(&self, concept_id: SctId) -> Option<&HashSet<SctId>> {
        self.descendants.get(&concept_id)
    }

    /// Gets all descendants of a concept, including self.
    pub fn get_descendants_or_self(&self, concept_id: SctId) -> HashSet<SctId> {
        let mut result = self
            .descendants
            .get(&concept_id)
            .cloned()
            .unwrap_or_default();
        result.insert(concept_id);
        result
    }

    /// Gets direct parents of a concept (O(1)).
    #[inline]
    pub fn get_direct_parents(&self, concept_id: SctId) -> &[SctId] {
        self.parents
            .get(&concept_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Gets direct children of a concept (O(1)).
    #[inline]
    pub fn get_direct_children(&self, concept_id: SctId) -> &[SctId] {
        self.children
            .get(&concept_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns true if the concept exists in the closure.
    #[inline]
    pub fn has_concept(&self, concept_id: SctId) -> bool {
        self.concepts.contains(&concept_id)
    }

    /// Returns the number of concepts in the closure.
    #[inline]
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Returns an iterator over all concept IDs.
    pub fn all_concepts(&self) -> impl Iterator<Item = SctId> + '_ {
        self.concepts.iter().copied()
    }

    /// Returns build statistics.
    pub fn stats(&self) -> &ClosureStats {
        &self.stats
    }

    /// Returns estimated memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        self.stats.memory_estimate_bytes
    }
}

/// Implement `EclQueryable` so the closure can be used directly with `EclExecutor`.
impl EclQueryable for TransitiveClosure {
    fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
        self.get_direct_children(concept_id).to_vec()
    }

    fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
        self.get_direct_parents(concept_id).to_vec()
    }

    fn has_concept(&self, concept_id: SctId) -> bool {
        self.concepts.contains(&concept_id)
    }

    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> {
        Box::new(self.concepts.iter().copied())
    }

    fn get_refset_members(&self, _refset_id: SctId) -> Vec<SctId> {
        // TransitiveClosure doesn't store refset data
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    /// Mock store for testing.
    struct MockStore {
        concepts: HashSet<SctId>,
        children: StdHashMap<SctId, Vec<SctId>>,
        parents: StdHashMap<SctId, Vec<SctId>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                concepts: HashSet::new(),
                children: StdHashMap::new(),
                parents: StdHashMap::new(),
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

    /// Creates a test hierarchy:
    /// ```text
    /// 100 (root)
    ///  |-- 200
    ///  |    |-- 400
    ///  |    |-- 500
    ///  |-- 300
    ///       |-- 600
    /// ```
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
    fn test_build_closure() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        assert_eq!(closure.concept_count(), 6);
        assert!(closure.stats().relationship_count > 0);
    }

    #[test]
    fn test_is_ancestor_of() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        // 100 is ancestor of all others
        assert!(closure.is_ancestor_of(100, 200));
        assert!(closure.is_ancestor_of(100, 400));
        assert!(closure.is_ancestor_of(100, 600));

        // 200 is ancestor of 400, 500
        assert!(closure.is_ancestor_of(200, 400));
        assert!(closure.is_ancestor_of(200, 500));
        assert!(!closure.is_ancestor_of(200, 600));

        // Not ancestor
        assert!(!closure.is_ancestor_of(400, 100));
        assert!(!closure.is_ancestor_of(300, 200));
    }

    #[test]
    fn test_is_descendant_of() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        // 400 is descendant of 200 and 100
        assert!(closure.is_descendant_of(400, 200));
        assert!(closure.is_descendant_of(400, 100));

        // 400 is not descendant of 300
        assert!(!closure.is_descendant_of(400, 300));
    }

    #[test]
    fn test_get_ancestors() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        // Ancestors of 400: {200, 100}
        let ancestors = closure.get_ancestors(400).unwrap();
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&200));
        assert!(ancestors.contains(&100));

        // 100 has no ancestors
        assert!(closure.get_ancestors(100).is_none());
    }

    #[test]
    fn test_get_descendants() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        // Descendants of 100: {200, 300, 400, 500, 600}
        let descendants = closure.get_descendants(100).unwrap();
        assert_eq!(descendants.len(), 5);

        // Descendants of 200: {400, 500}
        let descendants = closure.get_descendants(200).unwrap();
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&400));
        assert!(descendants.contains(&500));

        // 400 has no descendants
        assert!(closure.get_descendants(400).is_none());
    }

    #[test]
    fn test_get_ancestors_or_self() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        let ancestors = closure.get_ancestors_or_self(400);
        assert_eq!(ancestors.len(), 3); // 400, 200, 100
        assert!(ancestors.contains(&400));
        assert!(ancestors.contains(&200));
        assert!(ancestors.contains(&100));
    }

    #[test]
    fn test_get_descendants_or_self() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        let descendants = closure.get_descendants_or_self(200);
        assert_eq!(descendants.len(), 3); // 200, 400, 500
        assert!(descendants.contains(&200));
        assert!(descendants.contains(&400));
        assert!(descendants.contains(&500));
    }

    #[test]
    fn test_direct_parents_children() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        // Direct parents of 400: [200]
        let parents = closure.get_direct_parents(400);
        assert_eq!(parents.len(), 1);
        assert!(parents.contains(&200));

        // Direct children of 100: [200, 300]
        let children = closure.get_direct_children(100);
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_ecl_queryable_implementation() {
        let store = create_test_store();
        let closure = TransitiveClosure::build(&store);

        // Test EclQueryable methods
        assert!(closure.has_concept(100));
        assert!(!closure.has_concept(999));

        let children = EclQueryable::get_children(&closure, 100);
        assert_eq!(children.len(), 2);

        let parents = EclQueryable::get_parents(&closure, 400);
        assert_eq!(parents.len(), 1);
    }

    #[test]
    fn test_diamond_inheritance() {
        // Diamond pattern:
        //     100
        //    /   \
        //  200   300
        //    \   /
        //     400
        let mut store = MockStore::new();
        for id in [100, 200, 300, 400] {
            store.add_concept(id);
        }
        store.add_is_a(200, 100);
        store.add_is_a(300, 100);
        store.add_is_a(400, 200);
        store.add_is_a(400, 300);

        let closure = TransitiveClosure::build(&store);

        // 400 has ancestors: 200, 300, 100 (no duplicates)
        let ancestors = closure.get_ancestors(400).unwrap();
        assert_eq!(ancestors.len(), 3);
        assert!(ancestors.contains(&200));
        assert!(ancestors.contains(&300));
        assert!(ancestors.contains(&100));

        // 100 has descendants: 200, 300, 400
        let descendants = closure.get_descendants(100).unwrap();
        assert_eq!(descendants.len(), 3);
    }
}
