//! Hierarchy traversal for ECL execution.
//!
//! This module provides the `HierarchyTraverser` struct for traversing
//! the SNOMED CT concept hierarchy using BFS (Breadth-First Search).

use std::collections::{HashSet, VecDeque};

use snomed_types::SctId;

use crate::traits::EclQueryable;

/// Traverses SNOMED CT concept hierarchies using BFS.
///
/// The traverser provides efficient methods for:
/// - Getting all descendants of a concept
/// - Getting all ancestors of a concept
///
/// BFS is preferred over DFS because:
/// - More predictable memory usage for wide trees
/// - Better cache locality for typical SNOMED hierarchies
/// - Visits all nodes at current depth before going deeper
///
/// # Example
///
/// ```ignore
/// use snomed_ecl_executor::traverser::HierarchyTraverser;
///
/// let store = SnomedStore::new();
/// let traverser = HierarchyTraverser::new(&store);
///
/// // Get all descendants of Diabetes mellitus
/// let descendants = traverser.get_descendants(73211009);
/// ```
pub struct HierarchyTraverser<'a> {
    store: &'a dyn EclQueryable,
}

impl<'a> HierarchyTraverser<'a> {
    /// Creates a new hierarchy traverser with the given store.
    pub fn new(store: &'a dyn EclQueryable) -> Self {
        Self { store }
    }

    /// Gets all descendants of a concept using BFS traversal.
    ///
    /// This returns all concepts that are reachable by following
    /// child relationships (IS_A in reverse direction).
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get descendants of
    ///
    /// # Returns
    ///
    /// A HashSet containing all descendant concept IDs.
    /// Does NOT include the concept itself.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is number of descendants
    /// - Space complexity: O(n) for the visited set
    /// - Target: <100ms for 50K descendants
    pub fn get_descendants(&self, concept_id: SctId) -> HashSet<SctId> {
        // Pre-allocate with estimated capacity for performance
        let mut visited = HashSet::with_capacity(1000);
        let mut queue = VecDeque::with_capacity(100);

        // Start with the concept's direct children
        for child in self.store.get_children(concept_id) {
            if visited.insert(child) {
                queue.push_back(child);
            }
        }

        // BFS traversal
        while let Some(current) = queue.pop_front() {
            for child in self.store.get_children(current) {
                if visited.insert(child) {
                    queue.push_back(child);
                }
            }
        }

        visited
    }

    /// Gets all descendants of a concept, including the concept itself.
    ///
    /// This is equivalent to the `<<` (descendant or self) ECL operator.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get descendants of
    ///
    /// # Returns
    ///
    /// A HashSet containing the concept ID and all descendant concept IDs.
    pub fn get_descendants_or_self(&self, concept_id: SctId) -> HashSet<SctId> {
        let mut result = self.get_descendants(concept_id);
        result.insert(concept_id);
        result
    }

    /// Gets all ancestors of a concept using BFS traversal.
    ///
    /// This returns all concepts that are reachable by following
    /// parent relationships (IS_A direction).
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get ancestors of
    ///
    /// # Returns
    ///
    /// A HashSet containing all ancestor concept IDs.
    /// Does NOT include the concept itself.
    ///
    /// # Note
    ///
    /// SNOMED CT uses multiple inheritance (poly-hierarchy), so a concept
    /// can have multiple parents. This method correctly handles this case.
    pub fn get_ancestors(&self, concept_id: SctId) -> HashSet<SctId> {
        // Ancestor traversal typically returns fewer results than descendant
        let mut visited = HashSet::with_capacity(100);
        let mut queue = VecDeque::with_capacity(50);

        // Start with the concept's direct parents
        for parent in self.store.get_parents(concept_id) {
            if visited.insert(parent) {
                queue.push_back(parent);
            }
        }

        // BFS traversal
        while let Some(current) = queue.pop_front() {
            for parent in self.store.get_parents(current) {
                if visited.insert(parent) {
                    queue.push_back(parent);
                }
            }
        }

        visited
    }

    /// Gets all ancestors of a concept, including the concept itself.
    ///
    /// This is equivalent to the `>>` (ancestor or self) ECL operator.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get ancestors of
    ///
    /// # Returns
    ///
    /// A HashSet containing the concept ID and all ancestor concept IDs.
    pub fn get_ancestors_or_self(&self, concept_id: SctId) -> HashSet<SctId> {
        let mut result = self.get_ancestors(concept_id);
        result.insert(concept_id);
        result
    }

    /// Gets direct children of a concept.
    ///
    /// This is equivalent to the `<!` (child of) ECL operator.
    /// Returns only immediate children, not all descendants.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get children of
    ///
    /// # Returns
    ///
    /// A HashSet containing direct child concept IDs.
    pub fn get_direct_children(&self, concept_id: SctId) -> HashSet<SctId> {
        self.store.get_children(concept_id).into_iter().collect()
    }

    /// Gets direct parents of a concept.
    ///
    /// This is equivalent to the `>!` (parent of) ECL operator.
    /// Returns only immediate parents, not all ancestors.
    ///
    /// # Arguments
    ///
    /// * `concept_id` - The concept to get parents of
    ///
    /// # Returns
    ///
    /// A HashSet containing direct parent concept IDs.
    pub fn get_direct_parents(&self, concept_id: SctId) -> HashSet<SctId> {
        self.store.get_parents(concept_id).into_iter().collect()
    }

    /// Returns the number of concepts that would be traversed for a descendant query.
    ///
    /// Useful for estimating query cost without executing.
    pub fn count_descendants(&self, concept_id: SctId) -> usize {
        self.get_descendants(concept_id).len()
    }

    /// Returns the number of concepts that would be traversed for an ancestor query.
    ///
    /// Useful for estimating query cost without executing.
    pub fn count_ancestors(&self, concept_id: SctId) -> usize {
        self.get_ancestors(concept_id).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Mock store for testing hierarchy traversal.
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

    /// Creates a test hierarchy:
    /// ```text
    ///        100 (root)
    ///       /   \
    ///     200   300
    ///    /   \    \
    ///  400  500   600
    ///  /
    /// 700
    /// ```
    fn create_test_hierarchy() -> MockStore {
        let mut store = MockStore::new();

        for id in [100, 200, 300, 400, 500, 600, 700] {
            store.add_concept(id);
        }

        store.add_is_a(200, 100);
        store.add_is_a(300, 100);
        store.add_is_a(400, 200);
        store.add_is_a(500, 200);
        store.add_is_a(600, 300);
        store.add_is_a(700, 400);

        store
    }

    /// Creates a diamond inheritance pattern for testing poly-hierarchy:
    /// ```text
    ///     100
    ///    /   \
    ///  200   300
    ///    \   /
    ///     400
    /// ```
    fn create_diamond_hierarchy() -> MockStore {
        let mut store = MockStore::new();

        for id in [100, 200, 300, 400] {
            store.add_concept(id);
        }

        store.add_is_a(200, 100);
        store.add_is_a(300, 100);
        store.add_is_a(400, 200); // 400 IS_A 200
        store.add_is_a(400, 300); // 400 IS_A 300 (multiple inheritance)

        store
    }

    // Descendant tests

    #[test]
    fn test_get_descendants_single_level() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let descendants = traverser.get_descendants(100);

        // 100's descendants: 200, 300, 400, 500, 600, 700
        assert_eq!(descendants.len(), 6);
        assert!(descendants.contains(&200));
        assert!(descendants.contains(&300));
        assert!(descendants.contains(&400));
        assert!(descendants.contains(&500));
        assert!(descendants.contains(&600));
        assert!(descendants.contains(&700));
        // Should NOT contain self
        assert!(!descendants.contains(&100));
    }

    #[test]
    fn test_get_descendants_intermediate_node() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let descendants = traverser.get_descendants(200);

        // 200's descendants: 400, 500, 700
        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&400));
        assert!(descendants.contains(&500));
        assert!(descendants.contains(&700));
    }

    #[test]
    fn test_get_descendants_leaf_node() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let descendants = traverser.get_descendants(700);

        // 700 is a leaf, no descendants
        assert!(descendants.is_empty());
    }

    #[test]
    fn test_get_descendants_or_self() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let result = traverser.get_descendants_or_self(200);

        // Should include 200 and its descendants: 400, 500, 700
        assert_eq!(result.len(), 4);
        assert!(result.contains(&200)); // Self
        assert!(result.contains(&400));
        assert!(result.contains(&500));
        assert!(result.contains(&700));
    }

    #[test]
    fn test_get_descendants_empty_for_unknown() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let descendants = traverser.get_descendants(999);

        // Unknown concept has no descendants
        assert!(descendants.is_empty());
    }

    // Ancestor tests

    #[test]
    fn test_get_ancestors_single_level() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let ancestors = traverser.get_ancestors(200);

        // 200's ancestors: 100 only
        assert_eq!(ancestors.len(), 1);
        assert!(ancestors.contains(&100));
    }

    #[test]
    fn test_get_ancestors_multiple_levels() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let ancestors = traverser.get_ancestors(700);

        // 700's ancestors: 400, 200, 100
        assert_eq!(ancestors.len(), 3);
        assert!(ancestors.contains(&400));
        assert!(ancestors.contains(&200));
        assert!(ancestors.contains(&100));
        // Should NOT contain self
        assert!(!ancestors.contains(&700));
    }

    #[test]
    fn test_get_ancestors_root_node() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let ancestors = traverser.get_ancestors(100);

        // 100 is root, no ancestors
        assert!(ancestors.is_empty());
    }

    #[test]
    fn test_get_ancestors_or_self() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let result = traverser.get_ancestors_or_self(700);

        // Should include 700 and its ancestors: 400, 200, 100
        assert_eq!(result.len(), 4);
        assert!(result.contains(&700)); // Self
        assert!(result.contains(&400));
        assert!(result.contains(&200));
        assert!(result.contains(&100));
    }

    // Diamond (poly-hierarchy) tests

    #[test]
    fn test_diamond_ancestors_no_duplicates() {
        let store = create_diamond_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let ancestors = traverser.get_ancestors(400);

        // 400's ancestors: 200, 300, 100 (via both paths)
        // Should NOT have duplicates even though 100 is reached via two paths
        assert_eq!(ancestors.len(), 3);
        assert!(ancestors.contains(&200));
        assert!(ancestors.contains(&300));
        assert!(ancestors.contains(&100));
    }

    #[test]
    fn test_diamond_descendants_no_duplicates() {
        let store = create_diamond_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let descendants = traverser.get_descendants(100);

        // 100's descendants: 200, 300, 400
        // 400 should only appear once even though it's child of both 200 and 300
        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&200));
        assert!(descendants.contains(&300));
        assert!(descendants.contains(&400));
    }

    // Direct parent/child tests

    #[test]
    fn test_get_direct_children() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let children = traverser.get_direct_children(100);

        // 100's direct children: 200, 300 (not grandchildren)
        assert_eq!(children.len(), 2);
        assert!(children.contains(&200));
        assert!(children.contains(&300));
        assert!(!children.contains(&400)); // Not direct child
    }

    #[test]
    fn test_get_direct_parents() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let parents = traverser.get_direct_parents(400);

        // 400's direct parents: 200 only
        assert_eq!(parents.len(), 1);
        assert!(parents.contains(&200));
        assert!(!parents.contains(&100)); // Not direct parent
    }

    #[test]
    fn test_get_direct_parents_multiple() {
        let store = create_diamond_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let parents = traverser.get_direct_parents(400);

        // 400 has two direct parents: 200 and 300
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&200));
        assert!(parents.contains(&300));
    }

    #[test]
    fn test_get_direct_children_leaf() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let children = traverser.get_direct_children(700);

        // 700 is a leaf, no children
        assert!(children.is_empty());
    }

    #[test]
    fn test_get_direct_parents_root() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        let parents = traverser.get_direct_parents(100);

        // 100 is root, no parents
        assert!(parents.is_empty());
    }

    // Count tests

    #[test]
    fn test_count_descendants() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        assert_eq!(traverser.count_descendants(100), 6);
        assert_eq!(traverser.count_descendants(200), 3);
        assert_eq!(traverser.count_descendants(700), 0);
    }

    #[test]
    fn test_count_ancestors() {
        let store = create_test_hierarchy();
        let traverser = HierarchyTraverser::new(&store);

        assert_eq!(traverser.count_ancestors(100), 0);
        assert_eq!(traverser.count_ancestors(700), 3);
        assert_eq!(traverser.count_ancestors(400), 2);
    }

    // Performance test with deep hierarchy
    #[test]
    fn test_deep_hierarchy() {
        let mut store = MockStore::new();

        // Create a linear hierarchy with depth 100
        for i in 0..100 {
            store.add_concept(i);
            if i > 0 {
                store.add_is_a(i, i - 1);
            }
        }

        let traverser = HierarchyTraverser::new(&store);

        // Test descendant traversal from root
        let descendants = traverser.get_descendants(0);
        assert_eq!(descendants.len(), 99); // All except root

        // Test ancestor traversal from leaf
        let ancestors = traverser.get_ancestors(99);
        assert_eq!(ancestors.len(), 99); // All except leaf
    }

    // Performance test with wide hierarchy
    #[test]
    fn test_wide_hierarchy() {
        let mut store = MockStore::new();

        // Create a wide hierarchy: 1 root with 100 direct children
        store.add_concept(0);
        for i in 1..=100 {
            store.add_concept(i);
            store.add_is_a(i, 0);
        }

        let traverser = HierarchyTraverser::new(&store);

        let children = traverser.get_direct_children(0);
        assert_eq!(children.len(), 100);

        let descendants = traverser.get_descendants(0);
        assert_eq!(descendants.len(), 100);
    }
}
