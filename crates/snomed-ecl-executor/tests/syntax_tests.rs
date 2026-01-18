//! Integration tests for ECL syntax features.
//!
//! These tests verify that ECL syntax features (Phase 2) work correctly.

use snomed_ecl::parse;
use snomed_ecl_executor::{EclExecutor, EclQueryable, RelationshipInfo};
use std::collections::{HashMap, HashSet};

/// Mock SNOMED CT store for testing syntax features.
struct MockSyntaxStore {
    concepts: HashSet<u64>,
    parents: HashMap<u64, Vec<u64>>,
    children: HashMap<u64, Vec<u64>>,
    attributes: HashMap<u64, Vec<(u64, u64, u16)>>, // source -> (type, destination, group)
    refset_members: HashMap<u64, Vec<u64>>,
}

impl MockSyntaxStore {
    fn new() -> Self {
        let mut store = MockSyntaxStore {
            concepts: HashSet::new(),
            parents: HashMap::new(),
            children: HashMap::new(),
            attributes: HashMap::new(),
            refset_members: HashMap::new(),
        };

        // Add concepts
        // 138875005 - SNOMED CT Concept (root)
        store.concepts.insert(138875005);

        // 404684003 - Clinical finding
        store.concepts.insert(404684003);
        store.parents.insert(404684003, vec![138875005]);
        store.children.entry(138875005).or_default().push(404684003);

        // 73211009 - Diabetes mellitus
        store.concepts.insert(73211009);
        store.parents.insert(73211009, vec![404684003]);
        store.children.entry(404684003).or_default().push(73211009);

        // 64572001 - Disease
        store.concepts.insert(64572001);
        store.parents.insert(64572001, vec![404684003]);
        store.children.entry(404684003).or_default().push(64572001);

        // 386661006 - Fever
        store.concepts.insert(386661006);
        store.parents.insert(386661006, vec![404684003]);
        store.children.entry(404684003).or_default().push(386661006);

        // Attribute type
        // 116676008 - Associated morphology
        store.concepts.insert(116676008);

        // 363698007 - Finding site
        store.concepts.insert(363698007);

        // Body structures
        // 39057004 - Pulmonary valve
        store.concepts.insert(39057004);

        // 119186007 - Cardiac structure
        store.concepts.insert(119186007);

        // Add refset
        // 700043003 - Example problem list reference set
        store.concepts.insert(700043003);
        store.parents.insert(700043003, vec![138875005]);
        store
            .refset_members
            .insert(700043003, vec![73211009, 386661006]);

        // 723264001 - Lateralizable body structure reference set
        store.concepts.insert(723264001);
        store.parents.insert(723264001, vec![138875005]);
        store.refset_members.insert(723264001, vec![39057004]);

        store
    }
}

impl EclQueryable for MockSyntaxStore {
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
}

// ============================================================================
// Concept Reference Set Tests
// ============================================================================

// Note: ConceptSet syntax "(id1 id2)" is not yet implemented in the parser.
// These tests are marked as ignore until the feature is added.

#[test]
#[ignore = "ConceptSet syntax not yet implemented in parser"]
fn test_concept_reference_set_basic() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // Concept set with two IDs
    let result = executor.execute("(73211009 386661006)").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&386661006));
}

#[test]
#[ignore = "ConceptSet syntax not yet implemented in parser"]
fn test_concept_reference_set_with_invalid_id() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // Concept set with one invalid ID (should filter it out)
    let result = executor.execute("(73211009 999999999)").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&73211009));
}

#[test]
#[ignore = "ConceptSet syntax not yet implemented in parser"]
fn test_concept_reference_set_with_and() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // Concept set AND another expression
    let result = executor
        .execute("(73211009 386661006 64572001) AND << 404684003")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // All three are descendants of Clinical finding
    assert_eq!(ids.len(), 3);
}

// ============================================================================
// Enhanced MemberOf Tests
// ============================================================================

#[test]
fn test_enhanced_member_of_simple() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // Simple member-of
    let result = executor.execute("^ 700043003").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&386661006));
}

#[test]
#[ignore = "Enhanced MemberOf with nested expression needs non-refset concepts to return empty rather than error"]
fn test_enhanced_member_of_nested_expression() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // Members of any refset that is a child of root
    // (700043003 and 723264001 are both children of root)
    let result = executor.execute("^ (<! 138875005)").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Should include members from both refsets
    assert!(ids.contains(&73211009)); // From 700043003
    assert!(ids.contains(&386661006)); // From 700043003
    assert!(ids.contains(&39057004)); // From 723264001
}

// ============================================================================
// Alternate Identifier Tests
// ============================================================================

#[test]
fn test_alternate_identifier_snomed_uri() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // SNOMED CT URI
    let result = executor
        .execute("http://snomed.info/id/73211009")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&73211009));
}

#[test]
#[ignore = "Hierarchy operators with alternate identifiers not yet fully supported"]
fn test_alternate_identifier_with_hierarchy() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // Descendants of a concept specified by alternate identifier
    let result = executor
        .execute("<< http://snomed.info/id/404684003")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    // Should include Clinical finding and its descendants
    assert!(ids.contains(&404684003));
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&64572001));
    assert!(ids.contains(&386661006));
}

// ============================================================================
// Domain Prefix Filter Tests
// ============================================================================

#[test]
fn test_domain_prefix_concept() {
    let store = MockSyntaxStore::new();
    let executor = EclExecutor::new(&store);

    // Concept domain filter (active at concept level)
    // This should work the same as without domain prefix for now
    let result = executor.execute("* {{ C active = true }}").unwrap();

    // All concepts are considered active by default in MockSyntaxStore
    assert!(result.count() > 0);
}

// ============================================================================
// Numeric Comparison Tests (need concrete values in store)
// ============================================================================

// Note: Full numeric comparison tests require a mock store with concrete values.
// These tests verify the parsing works correctly.

#[test]
fn test_numeric_comparison_parsing() {
    // Verify parsing works
    let expr = parse("< 404684003 : 363698007 < #100").unwrap();
    assert!(matches!(
        expr,
        snomed_ecl::EclExpression::Refined { .. }
    ));
}

#[test]
fn test_boolean_concrete_value_parsing() {
    // Verify parsing works for boolean values
    let expr = parse("< 404684003 : 363698007 = #true").unwrap();
    assert!(matches!(
        expr,
        snomed_ecl::EclExpression::Refined { .. }
    ));
}

// ============================================================================
// Wildcard Term Matching Tests
// ============================================================================

#[test]
fn test_wildcard_term_filter_parsing() {
    // Verify wildcard term filter parsing
    let expr = parse(r#"<< 404684003 {{ term wild "diab*" }}"#).unwrap();
    match expr {
        snomed_ecl::EclExpression::Filtered { filters, .. } => {
            match &filters[0] {
                snomed_ecl::EclFilter::Term { match_type, .. } => {
                    assert!(matches!(match_type, snomed_ecl::TermMatchType::Wildcard));
                }
                _ => panic!("Expected Term filter"),
            }
        }
        _ => panic!("Expected Filtered expression"),
    }
}
