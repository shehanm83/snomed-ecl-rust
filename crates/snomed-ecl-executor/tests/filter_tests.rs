//! Integration tests for ECL filter execution.
//!
//! These tests verify that ECL filters work correctly with the executor.

use snomed_ecl_executor::{DescriptionInfo, EclExecutor, EclQueryable};
use std::collections::{HashMap, HashSet};

/// Mock SNOMED CT store for testing filters.
struct MockFilterStore {
    concepts: HashMap<u64, ConceptData>,
    descriptions: HashMap<u64, Vec<DescriptionData>>,
    refset_members: HashMap<u64, Vec<u64>>,
}

struct ConceptData {
    active: bool,
    definition_status_id: u64,
    module_id: u64,
    effective_time: Option<u32>,
}

struct DescriptionData {
    description_id: u64,
    term: String,
    language_code: String,
    type_id: u64,
    case_significance_id: u64,
}

impl MockFilterStore {
    fn new() -> Self {
        let mut store = MockFilterStore {
            concepts: HashMap::new(),
            descriptions: HashMap::new(),
            refset_members: HashMap::new(),
        };

        // Add test concepts
        // 404684003 - Clinical finding (active, defined)
        store.concepts.insert(
            404684003,
            ConceptData {
                active: true,
                definition_status_id: 900000000000073002, // Defined
                module_id: 900000000000207008,
                effective_time: Some(20200101),
            },
        );

        // 73211009 - Diabetes mellitus (active, primitive)
        store.concepts.insert(
            73211009,
            ConceptData {
                active: true,
                definition_status_id: 900000000000074008, // Primitive
                module_id: 900000000000207008,
                effective_time: Some(20190601),
            },
        );

        // 38341003 - Hypertension (inactive)
        store.concepts.insert(
            38341003,
            ConceptData {
                active: false,
                definition_status_id: 900000000000074008,
                module_id: 900000000000207008,
                effective_time: Some(20180101),
            },
        );

        // Add descriptions
        store.descriptions.insert(
            404684003,
            vec![
                DescriptionData {
                    description_id: 1,
                    term: "Clinical finding".to_string(),
                    language_code: "en".to_string(),
                    type_id: 900000000000003001, // FSN
                    case_significance_id: 900000000000448009, // Case insensitive
                },
                DescriptionData {
                    description_id: 2,
                    term: "Clinical finding".to_string(),
                    language_code: "en".to_string(),
                    type_id: 900000000000013009, // Synonym
                    case_significance_id: 900000000000448009,
                },
            ],
        );

        store.descriptions.insert(
            73211009,
            vec![
                DescriptionData {
                    description_id: 3,
                    term: "Diabetes mellitus (disorder)".to_string(),
                    language_code: "en".to_string(),
                    type_id: 900000000000003001, // FSN
                    case_significance_id: 900000000000017005, // Case sensitive
                },
                DescriptionData {
                    description_id: 4,
                    term: "Diabetes".to_string(),
                    language_code: "en".to_string(),
                    type_id: 900000000000013009, // Synonym
                    case_significance_id: 900000000000448009,
                },
            ],
        );

        // Add refset members
        store
            .refset_members
            .insert(700043003, vec![404684003, 73211009]);

        store
    }
}

impl EclQueryable for MockFilterStore {
    fn has_concept(&self, concept_id: u64) -> bool {
        self.concepts.contains_key(&concept_id)
    }

    fn get_parents(&self, _concept_id: u64) -> Vec<u64> {
        Vec::new()
    }

    fn get_children(&self, _concept_id: u64) -> Vec<u64> {
        Vec::new()
    }

    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = u64> + '_> {
        Box::new(self.concepts.keys().copied())
    }

    fn get_refset_members(&self, refset_id: u64) -> Vec<u64> {
        self.refset_members.get(&refset_id).cloned().unwrap_or_default()
    }

    fn is_concept_active(&self, concept_id: u64) -> bool {
        self.concepts
            .get(&concept_id)
            .map(|c| c.active)
            .unwrap_or(false)
    }

    fn is_concept_primitive(&self, concept_id: u64) -> Option<bool> {
        self.concepts
            .get(&concept_id)
            .map(|c| c.definition_status_id == 900000000000074008)
    }

    fn get_concept_module(&self, concept_id: u64) -> Option<u64> {
        self.concepts.get(&concept_id).map(|c| c.module_id)
    }

    fn get_concept_effective_time(&self, concept_id: u64) -> Option<u32> {
        self.concepts
            .get(&concept_id)
            .and_then(|c| c.effective_time)
    }

    fn get_descriptions(&self, concept_id: u64) -> Vec<DescriptionInfo> {
        self.descriptions
            .get(&concept_id)
            .map(|descs| {
                descs
                    .iter()
                    .map(|d| DescriptionInfo {
                        description_id: d.description_id,
                        term: d.term.clone(),
                        language_code: d.language_code.clone(),
                        type_id: d.type_id,
                        case_significance_id: d.case_significance_id,
                        active: true,
                        module_id: 900000000000207008,
                        effective_time: None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[test]
fn test_active_filter_true() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // All active concepts
    let result = executor.execute("* {{ active = true }}").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&404684003));
    assert!(ids.contains(&73211009));
    assert!(!ids.contains(&38341003)); // Inactive
}

#[test]
fn test_active_filter_false() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // All inactive concepts
    let result = executor.execute("* {{ active = false }}").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(!ids.contains(&404684003));
    assert!(!ids.contains(&73211009));
    assert!(ids.contains(&38341003)); // Inactive
}

#[test]
fn test_definition_status_filter_primitive() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Primitive concepts only
    let result = executor
        .execute("* {{ definitionStatus = primitive }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(!ids.contains(&404684003)); // Defined
    assert!(ids.contains(&73211009)); // Primitive
    assert!(ids.contains(&38341003)); // Primitive
}

#[test]
fn test_definition_status_filter_defined() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Defined concepts only
    let result = executor
        .execute("* {{ definitionStatus = defined }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&404684003)); // Defined
    assert!(!ids.contains(&73211009)); // Primitive
    assert!(!ids.contains(&38341003)); // Primitive
}

#[test]
fn test_module_filter() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Concepts in the international module
    let result = executor
        .execute("* {{ moduleId = 900000000000207008 }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&404684003));
    assert!(ids.contains(&73211009));
    assert!(ids.contains(&38341003));
}

#[test]
fn test_effective_time_filter() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Concepts effective from 2019 onwards
    let result = executor
        .execute("* {{ effectiveTime >= 20190101 }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(ids.contains(&404684003)); // 20200101
    assert!(ids.contains(&73211009)); // 20190601
    assert!(!ids.contains(&38341003)); // 20180101
}

#[test]
fn test_term_filter_contains() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Concepts with descriptions containing "diabetes"
    let result = executor.execute(r#"* {{ term = "diabetes" }}"#).unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(!ids.contains(&404684003));
    assert!(ids.contains(&73211009)); // Has "Diabetes mellitus" and "Diabetes"
}

#[test]
fn test_combined_filters() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Active primitive concepts
    let result = executor
        .execute("* {{ active = true, definitionStatus = primitive }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert!(!ids.contains(&404684003)); // Defined
    assert!(ids.contains(&73211009)); // Active and primitive
    assert!(!ids.contains(&38341003)); // Inactive
}

#[test]
fn test_id_filter_single() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Filter to specific ID
    let result = executor.execute("* {{ id = 73211009 }}").unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&73211009));
}

#[test]
fn test_id_filter_multiple() {
    let store = MockFilterStore::new();
    let executor = EclExecutor::new(&store);

    // Filter to multiple IDs
    let result = executor
        .execute("* {{ id = (404684003 73211009) }}")
        .unwrap();
    let ids: HashSet<_> = result.iter().collect();

    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&404684003));
    assert!(ids.contains(&73211009));
}
