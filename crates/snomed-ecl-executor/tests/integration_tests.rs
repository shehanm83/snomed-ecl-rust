//! Comprehensive integration tests for ECL executor.
//!
//! These tests cover ECL 2.2 compliance and various edge cases.

use snomed_ecl_executor::{EclExecutor, EclQueryable, RelationshipInfo};
use std::collections::{HashMap, HashSet};

/// Comprehensive mock SNOMED CT store for integration testing.
struct IntegrationTestStore {
    concepts: HashSet<u64>,
    parents: HashMap<u64, Vec<u64>>,
    children: HashMap<u64, Vec<u64>>,
    attributes: HashMap<u64, Vec<(u64, u64, u16)>>,
    inbound_attributes: HashMap<u64, Vec<(u64, u64, u16)>>, // destination -> (type, source, group)
    refset_members: HashMap<u64, Vec<u64>>,
    active_concepts: HashSet<u64>,
    primitive_concepts: HashSet<u64>,
}

impl IntegrationTestStore {
    fn new() -> Self {
        let mut store = IntegrationTestStore {
            concepts: HashSet::new(),
            parents: HashMap::new(),
            children: HashMap::new(),
            attributes: HashMap::new(),
            inbound_attributes: HashMap::new(),
            refset_members: HashMap::new(),
            active_concepts: HashSet::new(),
            primitive_concepts: HashSet::new(),
        };

        // Build a realistic hierarchy
        // 138875005 - SNOMED CT Concept (root)
        store.add_concept(138875005, true, false);

        // 404684003 - Clinical finding
        store.add_concept(404684003, true, false);
        store.add_parent(404684003, 138875005);

        // 64572001 - Disease (defined, not primitive)
        store.add_concept(64572001, true, false);
        store.add_parent(64572001, 404684003);

        // 73211009 - Diabetes mellitus
        store.add_concept(73211009, true, true);
        store.add_parent(73211009, 64572001);

        // 46635009 - Type 1 diabetes
        store.add_concept(46635009, true, true);
        store.add_parent(46635009, 73211009);

        // 44054006 - Type 2 diabetes
        store.add_concept(44054006, true, true);
        store.add_parent(44054006, 73211009);

        // 386661006 - Fever
        store.add_concept(386661006, true, true);
        store.add_parent(386661006, 404684003);

        // 38341003 - Hypertension (inactive)
        store.add_concept(38341003, false, true);
        store.add_parent(38341003, 64572001);

        // Body structures
        // 123037004 - Body structure
        store.add_concept(123037004, true, false);
        store.add_parent(123037004, 138875005);

        // 39057004 - Pulmonary valve
        store.add_concept(39057004, true, true);
        store.add_parent(39057004, 123037004);

        // 80891009 - Heart structure
        store.add_concept(80891009, true, true);
        store.add_parent(80891009, 123037004);

        // Attribute types
        // 363698007 - Finding site
        store.add_concept(363698007, true, false);
        // 116676008 - Associated morphology
        store.add_concept(116676008, true, false);

        // Add some attributes
        // Diabetes has finding site = pancreas (not in our store, use heart for simplicity)
        store.add_attribute(73211009, 363698007, 80891009, 0);

        // Add refsets
        // 700043003 - Problem list refset
        store.add_concept(700043003, true, false);
        store.refset_members.insert(700043003, vec![73211009, 386661006]);

        // 723264001 - Another refset
        store.add_concept(723264001, true, false);
        store.refset_members.insert(723264001, vec![46635009, 44054006]);

        store
    }

    fn add_concept(&mut self, id: u64, active: bool, primitive: bool) {
        self.concepts.insert(id);
        if active {
            self.active_concepts.insert(id);
        }
        if primitive {
            self.primitive_concepts.insert(id);
        }
    }

    fn add_parent(&mut self, child: u64, parent: u64) {
        self.parents.entry(child).or_default().push(parent);
        self.children.entry(parent).or_default().push(child);
    }

    fn add_attribute(&mut self, source: u64, type_id: u64, dest: u64, group: u16) {
        self.attributes
            .entry(source)
            .or_default()
            .push((type_id, dest, group));
        self.inbound_attributes
            .entry(dest)
            .or_default()
            .push((type_id, source, group));
    }
}

impl EclQueryable for IntegrationTestStore {
    fn has_concept(&self, concept_id: u64) -> bool {
        self.concepts.contains(&concept_id)
    }

    fn get_parents(&self, concept_id: u64) -> Vec<u64> {
        self.parents.get(&concept_id).cloned().unwrap_or_default()
    }

    fn get_children(&self, concept_id: u64) -> Vec<u64> {
        self.children.get(&concept_id).cloned().unwrap_or_default()
    }

    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = u64> + '_> {
        Box::new(self.concepts.iter().copied())
    }

    fn get_refset_members(&self, refset_id: u64) -> Vec<u64> {
        self.refset_members.get(&refset_id).cloned().unwrap_or_default()
    }

    fn get_attributes(&self, concept_id: u64) -> Vec<RelationshipInfo> {
        self.attributes
            .get(&concept_id)
            .map(|attrs| {
                attrs
                    .iter()
                    .map(|(type_id, dest_id, group)| RelationshipInfo {
                        type_id: *type_id,
                        destination_id: *dest_id,
                        group: *group,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_inbound_relationships(&self, concept_id: u64) -> Vec<RelationshipInfo> {
        self.inbound_attributes
            .get(&concept_id)
            .map(|attrs| {
                attrs
                    .iter()
                    .map(|(type_id, _source_id, group)| RelationshipInfo {
                        type_id: *type_id,
                        destination_id: concept_id,
                        group: *group,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn is_concept_active(&self, concept_id: u64) -> bool {
        self.active_concepts.contains(&concept_id)
    }

    fn is_concept_primitive(&self, concept_id: u64) -> Option<bool> {
        if self.concepts.contains(&concept_id) {
            Some(self.primitive_concepts.contains(&concept_id))
        } else {
            None
        }
    }
}

// ============================================================================
// Basic Hierarchy Tests
// ============================================================================

#[test]
fn test_self_concept() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    let result = executor.execute("73211009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&73211009));
}

#[test]
fn test_descendant_of() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Descendants of Diabetes mellitus (not including self)
    let result = executor.execute("< 73211009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&46635009)); // Type 1
    assert!(ids.contains(&44054006)); // Type 2
    assert!(!ids.contains(&73211009)); // Self excluded
}

#[test]
fn test_descendant_or_self_of() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Descendants of Diabetes mellitus (including self)
    let result = executor.execute("<< 73211009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&73211009)); // Self
    assert!(ids.contains(&46635009)); // Type 1
    assert!(ids.contains(&44054006)); // Type 2
}

#[test]
fn test_ancestor_of() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Ancestors of Type 1 diabetes
    let result = executor.execute("> 46635009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&73211009)); // Diabetes mellitus
    assert!(ids.contains(&64572001)); // Disease
    assert!(ids.contains(&404684003)); // Clinical finding
    assert!(ids.contains(&138875005)); // Root
    assert!(!ids.contains(&46635009)); // Self excluded
}

#[test]
fn test_child_of() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Direct children of Diabetes mellitus
    let result = executor.execute("<! 73211009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&46635009)); // Type 1
    assert!(ids.contains(&44054006)); // Type 2
}

#[test]
fn test_parent_of() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Direct parents of Type 1 diabetes
    let result = executor.execute(">! 46635009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&73211009)); // Diabetes mellitus
}

// ============================================================================
// Compound Expression Tests
// ============================================================================

#[test]
fn test_and_expression() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Descendants of Disease AND descendants of Clinical finding
    let result = executor
        .execute("<< 64572001 AND << 404684003")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Disease is a Clinical finding, so all descendants of Disease are in both
    assert!(ids.contains(&64572001)); // Disease
    assert!(ids.contains(&73211009)); // Diabetes
    assert!(ids.contains(&46635009)); // Type 1
    assert!(ids.contains(&44054006)); // Type 2
}

#[test]
fn test_or_expression() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Fever OR diabetes descendants
    let result = executor
        .execute("386661006 OR << 73211009")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&386661006)); // Fever
    assert!(ids.contains(&73211009)); // Diabetes
    assert!(ids.contains(&46635009)); // Type 1
    assert!(ids.contains(&44054006)); // Type 2
}

#[test]
fn test_minus_expression() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // All diabetes minus Type 1
    let result = executor
        .execute("<< 73211009 MINUS 46635009")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&73211009)); // Diabetes
    assert!(ids.contains(&44054006)); // Type 2
    assert!(!ids.contains(&46635009)); // Type 1 excluded
}

// ============================================================================
// MemberOf Tests
// ============================================================================

#[test]
fn test_member_of_simple() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    let result = executor.execute("^ 700043003").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&386661006));
}

#[test]
fn test_member_of_with_and() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Members of refset that are also descendants of Disease
    let result = executor
        .execute("^ 700043003 AND << 64572001")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Only diabetes is both a member and a descendant of Disease
    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&73211009));
}

// ============================================================================
// Refinement Tests
// ============================================================================

#[test]
fn test_simple_refinement() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Concepts with finding site = heart structure
    let result = executor
        .execute("* : 363698007 = 80891009")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Diabetes has finding site = heart in our mock
    assert!(ids.contains(&73211009));
}

#[test]
fn test_refinement_with_hierarchy() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Clinical findings with finding site = descendant of body structure
    let result = executor
        .execute("<< 404684003 : 363698007 = << 123037004")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Should include diabetes (has finding site = heart, which is under body structure)
    assert!(ids.contains(&73211009));
}

// ============================================================================
// Reverse Attribute Tests
// ============================================================================

#[test]
fn test_reverse_attribute() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Body structures that are the finding site of something
    // Uses R (reverse) flag
    let result = executor
        .execute("<< 123037004 : R 363698007 = *")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Heart structure is the finding site of diabetes
    assert!(ids.contains(&80891009));
}

// ============================================================================
// Filter Tests
// ============================================================================

#[test]
fn test_active_filter() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // All active descendants of Disease
    let result = executor
        .execute("<< 64572001 {{ active = true }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&64572001));
    assert!(ids.contains(&73211009));
    assert!(!ids.contains(&38341003)); // Inactive
}

#[test]
fn test_definition_status_filter() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // All primitive descendants of Disease
    let result = executor
        .execute("<< 64572001 {{ definitionStatus = primitive }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // All descendants except Disease itself are primitive in our mock
    assert!(!ids.contains(&64572001)); // Defined (not primitive)
    assert!(ids.contains(&73211009)); // Primitive
    assert!(ids.contains(&46635009)); // Primitive
}

// ============================================================================
// Complex Query Tests
// ============================================================================

#[test]
fn test_complex_query() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Active primitive descendants of clinical finding with finding site
    let result = executor
        .execute(
            "<< 404684003 {{ active = true, definitionStatus = primitive }} : 363698007 = *",
        )
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Diabetes is active, primitive, and has a finding site
    assert!(ids.contains(&73211009));
}

#[test]
fn test_nested_expressions() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // (Diabetes descendants OR Fever) AND Clinical findings
    let result = executor
        .execute("(<< 73211009 OR 386661006) AND << 404684003")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // All are clinical findings
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&46635009));
    assert!(ids.contains(&44054006));
    assert!(ids.contains(&386661006));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_result() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Descendants of a concept with no children
    let result = executor.execute("< 46635009").unwrap();

    assert_eq!(result.count(), 0);
}

#[test]
fn test_invalid_concept() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Non-existent concept
    let result = executor.execute("999999999");

    // Should return error for concept not found
    assert!(result.is_err() || result.unwrap().count() == 0);
}

#[test]
fn test_wildcard() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // All concepts
    let result = executor.execute("*").unwrap();

    // Should return all concepts in the store
    assert!(result.count() > 10);
}

// ============================================================================
// ECL Specification Compliance Tests
// ============================================================================

#[test]
fn test_multiple_hierarchy_levels() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Test that descendants correctly traverse multiple levels
    // Root -> Clinical finding -> Disease -> Diabetes -> Type 1/Type 2
    let result = executor.execute("< 138875005").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Should include all descendants at all levels
    assert!(ids.contains(&404684003)); // Clinical finding (child of root)
    assert!(ids.contains(&73211009)); // Diabetes (grandchild)
    assert!(ids.contains(&46635009)); // Type 1 (great-grandchild)
}

#[test]
fn test_ancestor_of_multiple_levels() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Ancestors of Type 2 diabetes should include all levels up to root
    let result = executor.execute(">> 44054006").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&44054006)); // Self (ancestor-or-self)
    assert!(ids.contains(&73211009)); // Diabetes (parent)
    assert!(ids.contains(&64572001)); // Disease (grandparent)
    assert!(ids.contains(&404684003)); // Clinical finding
    assert!(ids.contains(&138875005)); // Root
}

#[test]
fn test_conjunction_with_hierarchy() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // (Descendants of root) AND (Ancestors of Type 1 diabetes)
    // Should only include ancestors of Type 1 that are also descendants of root
    let result = executor
        .execute("< 138875005 AND > 46635009")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Clinical finding, Disease, and Diabetes are both descendants of root
    // and ancestors of Type 1
    assert!(ids.contains(&404684003));
    assert!(ids.contains(&64572001));
    assert!(ids.contains(&73211009));
    // Root is NOT a descendant of itself
    assert!(!ids.contains(&138875005));
}

#[test]
fn test_disjunction_union() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Children of diabetes OR children of body structure
    let result = executor
        .execute("<! 73211009 OR <! 123037004")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Type 1 and Type 2 are children of diabetes
    assert!(ids.contains(&46635009));
    assert!(ids.contains(&44054006));
    // Pulmonary valve and Heart are children of body structure
    assert!(ids.contains(&39057004));
    assert!(ids.contains(&80891009));
}

#[test]
fn test_exclusion_difference() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // All descendants of Clinical finding MINUS all descendants of Disease
    let result = executor
        .execute("<< 404684003 MINUS << 64572001")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Clinical finding itself should be included (not a descendant of Disease)
    assert!(ids.contains(&404684003));
    // Fever should be included (not under Disease)
    assert!(ids.contains(&386661006));
    // Diabetes should NOT be included (is descendant of Disease)
    assert!(!ids.contains(&73211009));
    // Type 1 should NOT be included
    assert!(!ids.contains(&46635009));
}

#[test]
fn test_deeply_nested_parentheses() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // ((Type 1 OR Type 2) AND descendants of diabetes)
    let result = executor
        .execute("((46635009 OR 44054006) AND << 73211009)")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Both Type 1 and Type 2 are descendants of diabetes
    assert!(ids.contains(&46635009));
    assert!(ids.contains(&44054006));
    assert_eq!(ids.len(), 2);
}

#[test]
fn test_member_of_intersection() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Members of problem list refset that are diabetes types
    let result = executor
        .execute("^ 723264001 AND << 73211009")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // 723264001 contains Type 1 and Type 2
    // Both are descendants of diabetes
    assert!(ids.contains(&46635009));
    assert!(ids.contains(&44054006));
}

#[test]
fn test_attribute_with_wildcard_value() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Any concept with any finding site
    let result = executor.execute("* : 363698007 = *").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Diabetes has finding site = heart
    assert!(ids.contains(&73211009));
}

#[test]
fn test_combined_filters_active_and_primitive() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Active AND primitive descendants of clinical finding
    let result = executor
        .execute("<< 404684003 {{ active = true, definitionStatus = primitive }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Should include active primitive concepts only
    // Diabetes, Type 1, Type 2, Fever are all active and primitive
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&46635009));
    assert!(ids.contains(&386661006));
    // Hypertension is inactive
    assert!(!ids.contains(&38341003));
    // Disease is defined (not primitive)
    assert!(!ids.contains(&64572001));
}

#[test]
fn test_child_or_self_of() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Child-or-self of diabetes
    let result = executor.execute("<<! 73211009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Should include diabetes itself plus direct children
    assert!(ids.contains(&73211009)); // Self
    assert!(ids.contains(&46635009)); // Type 1 (child)
    assert!(ids.contains(&44054006)); // Type 2 (child)
    assert_eq!(ids.len(), 3);
}

#[test]
fn test_parent_or_self_of() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Parent-or-self of Type 1 diabetes
    let result = executor.execute(">>! 46635009").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Should include Type 1 itself plus direct parent
    assert!(ids.contains(&46635009)); // Self
    assert!(ids.contains(&73211009)); // Diabetes (parent)
    assert_eq!(ids.len(), 2);
}

#[test]
fn test_multiple_member_of_refsets() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Members of first refset OR members of second refset
    let result = executor
        .execute("^ 700043003 OR ^ 723264001")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // 700043003 contains: Diabetes, Fever
    // 723264001 contains: Type 1, Type 2
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&386661006));
    assert!(ids.contains(&46635009));
    assert!(ids.contains(&44054006));
}

#[test]
fn test_refinement_with_concept_value() {
    let store = IntegrationTestStore::new();
    let executor = EclExecutor::new(&store);

    // Clinical findings with finding site = heart structure exactly
    let result = executor
        .execute("<< 404684003 : 363698007 = 80891009")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Diabetes has finding site = heart
    assert!(ids.contains(&73211009));
}
