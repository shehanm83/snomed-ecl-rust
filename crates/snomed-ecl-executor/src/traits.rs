//! Traits for ECL query execution.
//!
//! This module defines the [`EclQueryable`] trait that must be implemented
//! by any SNOMED CT store that wants to execute ECL queries.
//!
//! # Implementing EclQueryable
//!
//! To use the ECL executor with your SNOMED CT store, implement the
//! [`EclQueryable`] trait:
//!
//! ```ignore
//! use snomed_ecl_executor::{EclQueryable, EclExecutor, SctId};
//!
//! struct MyStore {
//!     // Your store implementation...
//! }
//!
//! impl EclQueryable for MyStore {
//!     fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
//!         // Return direct children (concepts with IS_A relationship to this concept)
//!         todo!()
//!     }
//!
//!     fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
//!         // Return direct parents (this concept has IS_A relationship to them)
//!         todo!()
//!     }
//!
//!     fn has_concept(&self, concept_id: SctId) -> bool {
//!         // Return true if concept exists in the store
//!         todo!()
//!     }
//!
//!     fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> {
//!         // Return iterator over all concept IDs (for wildcard queries)
//!         todo!()
//!     }
//!
//!     fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId> {
//!         // Return members of a reference set (for ^ queries)
//!         Vec::new() // Return empty if not supported
//!     }
//! }
//!
//! // Now you can use EclExecutor with your store
//! let store = MyStore { /* ... */ };
//! let executor = EclExecutor::new(&store);
//! let result = executor.execute("< 73211009")?;
//! ```

use snomed_ecl::SctId;

// =============================================================================
// Relationship Info (for attribute queries)
// =============================================================================

/// Information about a relationship for attribute queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipInfo {
    /// The relationship type (attribute type).
    pub type_id: SctId,
    /// The destination concept.
    pub destination_id: SctId,
    /// The relationship group number (0 = ungrouped).
    pub group: u16,
}

/// Concrete value for concrete domain relationships.
#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteValueRef {
    /// Integer value.
    Integer(i64),
    /// Decimal value.
    Decimal(f64),
    /// String value.
    String(String),
}

/// Information about a concrete relationship.
#[derive(Debug, Clone, PartialEq)]
pub struct ConcreteRelationshipInfo {
    /// The relationship type (attribute type).
    pub type_id: SctId,
    /// The concrete value.
    pub value: ConcreteValueRef,
    /// The relationship group number (0 = ungrouped).
    pub group: u16,
}

/// Description information for term filtering.
#[derive(Debug, Clone)]
pub struct DescriptionInfo {
    /// The description ID.
    pub description_id: SctId,
    /// The description term.
    pub term: String,
    /// The language code (e.g., "en").
    pub language_code: String,
    /// The description type ID (FSN, synonym, etc.).
    /// - 900000000000003001 = Fully specified name
    /// - 900000000000013009 = Synonym
    /// - 900000000000550004 = Definition
    pub type_id: SctId,
    /// Case significance ID.
    /// - 900000000000448009 = Case insensitive
    /// - 900000000000017005 = Case sensitive (initial character)
    /// - 900000000000020002 = Case sensitive (entire term)
    pub case_significance_id: SctId,
    /// Whether the description is active.
    pub active: bool,
    /// Effective time in YYYYMMDD format.
    pub effective_time: Option<u32>,
    /// Module ID.
    pub module_id: SctId,
}

/// Acceptability of a description in a language reference set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acceptability {
    /// Preferred term in this dialect.
    Preferred,
    /// Acceptable term in this dialect.
    Acceptable,
}

/// Language reference set membership for a description.
#[derive(Debug, Clone)]
pub struct LanguageRefsetMember {
    /// The language reference set ID (dialect).
    pub refset_id: SctId,
    /// Acceptability in this refset.
    pub acceptability: Acceptability,
}

/// Concept metadata for filtering.
#[derive(Debug, Clone)]
pub struct ConceptInfo {
    /// Whether the concept is primitive (true) or fully defined (false).
    pub is_primitive: bool,
    /// Module ID.
    pub module_id: SctId,
    /// Effective time in YYYYMMDD format.
    pub effective_time: Option<u32>,
    /// Whether the concept is active.
    pub active: bool,
}

/// Trait for stores that can be queried with ECL expressions.
///
/// This trait abstracts the underlying SNOMED store implementation,
/// allowing the executor to work with different store implementations.
///
/// Implement this trait for your store type in your application crate.
/// See the module-level documentation for a complete example.
///
/// # Required Methods
///
/// - [`get_children`](Self::get_children) - Get direct children via IS_A
/// - [`get_parents`](Self::get_parents) - Get direct parents via IS_A
/// - [`has_concept`](Self::has_concept) - Check if concept exists
/// - [`all_concept_ids`](Self::all_concept_ids) - Iterate all concepts (for wildcards)
/// - [`get_refset_members`](Self::get_refset_members) - Get reference set members
///
/// # Optional Methods (with defaults)
///
/// Advanced ECL features have default implementations that return empty results.
/// Override them to support attribute refinements, filters, etc.
pub trait EclQueryable: Send + Sync {
    /// Gets direct children of a concept (via IS_A relationships).
    ///
    /// Returns an empty Vec if the concept has no children or doesn't exist.
    fn get_children(&self, concept_id: SctId) -> Vec<SctId>;

    /// Gets direct parents of a concept (via IS_A relationships).
    ///
    /// Returns an empty Vec if the concept has no parents or doesn't exist.
    fn get_parents(&self, concept_id: SctId) -> Vec<SctId>;

    /// Checks if a concept exists in the store.
    fn has_concept(&self, concept_id: SctId) -> bool;

    /// Returns an iterator over all concept IDs in the store.
    ///
    /// Used for wildcard (*) queries.
    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_>;

    /// Gets members of a reference set.
    ///
    /// Returns an empty Vec if the reference set doesn't exist or has no members.
    fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId>;

    // =========================================================================
    // Advanced ECL Features (Story 10.9)
    // =========================================================================

    /// Gets attribute relationships for a concept (non-IS_A relationships).
    ///
    /// Returns all relationships where the source is the given concept,
    /// excluding IS_A relationships which are handled via get_parents/get_children.
    ///
    /// Used for attribute refinement queries like:
    /// `< 404684003 : 363698007 = << 39057004`
    fn get_attributes(&self, concept_id: SctId) -> Vec<RelationshipInfo> {
        // Default implementation returns empty - stores can override
        let _ = concept_id;
        Vec::new()
    }

    /// Gets concepts that have a specific attribute with a specific target value.
    ///
    /// Returns concepts where:
    /// - The concept has a relationship of type `attribute_type_id`
    /// - The relationship's destination is `target_id`
    ///
    /// Used for reverse attribute lookups in dot notation.
    fn get_concepts_with_attribute(&self, attribute_type_id: SctId, target_id: SctId) -> Vec<SctId> {
        let _ = (attribute_type_id, target_id);
        Vec::new()
    }

    /// Gets concrete domain values for a concept.
    ///
    /// Returns concrete relationships (numeric/string values) for the concept.
    fn get_concrete_values(&self, concept_id: SctId) -> Vec<ConcreteRelationshipInfo> {
        let _ = concept_id;
        Vec::new()
    }

    /// Gets descriptions for a concept (for term filtering).
    ///
    /// Returns all descriptions associated with the concept.
    fn get_descriptions(&self, concept_id: SctId) -> Vec<DescriptionInfo> {
        let _ = concept_id;
        Vec::new()
    }

    /// Gets the preferred term for a concept.
    ///
    /// Returns the preferred synonym for display purposes.
    fn get_preferred_term(&self, concept_id: SctId) -> Option<String> {
        let _ = concept_id;
        None
    }

    /// Gets inactive concepts that were replaced by the given concept.
    ///
    /// Used for history supplement queries.
    fn get_historical_associations(&self, concept_id: SctId) -> Vec<SctId> {
        let _ = concept_id;
        Vec::new()
    }

    /// Checks if a concept is active.
    fn is_concept_active(&self, concept_id: SctId) -> bool {
        // Default: assume active if concept exists
        self.has_concept(concept_id)
    }

    /// Gets the module ID for a concept.
    fn get_concept_module(&self, concept_id: SctId) -> Option<SctId> {
        let _ = concept_id;
        None
    }

    // =========================================================================
    // Filter Support Methods (ECL 2.2)
    // =========================================================================

    /// Gets full concept metadata for filtering.
    ///
    /// Returns definition status, module, effective time, and active status.
    fn get_concept_info(&self, concept_id: SctId) -> Option<ConceptInfo> {
        let _ = concept_id;
        None
    }

    /// Gets language reference set memberships for a description.
    ///
    /// Returns which language reference sets the description belongs to
    /// and its acceptability (preferred/acceptable) in each.
    fn get_description_language_refsets(&self, description_id: SctId) -> Vec<LanguageRefsetMember> {
        let _ = description_id;
        Vec::new()
    }

    /// Gets the semantic tag for a concept (extracted from FSN).
    ///
    /// The semantic tag is the text in parentheses at the end of the FSN.
    /// For example, "Diabetes mellitus (disorder)" has semantic tag "disorder".
    fn get_semantic_tag(&self, concept_id: SctId) -> Option<String> {
        // Default implementation: try to extract from FSN
        let descriptions = self.get_descriptions(concept_id);
        for desc in descriptions {
            // FSN type ID
            if desc.type_id == 900000000000003001 {
                // Extract semantic tag from parentheses
                if let Some(start) = desc.term.rfind('(') {
                    if let Some(end) = desc.term.rfind(')') {
                        if start < end {
                            return Some(desc.term[start + 1..end].to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Gets the effective time for a concept in YYYYMMDD format.
    fn get_concept_effective_time(&self, concept_id: SctId) -> Option<u32> {
        self.get_concept_info(concept_id).and_then(|info| info.effective_time)
    }

    /// Checks if a concept is primitive (vs. fully defined).
    fn is_concept_primitive(&self, concept_id: SctId) -> Option<bool> {
        self.get_concept_info(concept_id).map(|info| info.is_primitive)
    }

    /// Gets inbound relationships (where this concept is the destination).
    ///
    /// Used for reverse attribute queries (R flag in ECL).
    fn get_inbound_relationships(&self, concept_id: SctId) -> Vec<RelationshipInfo> {
        let _ = concept_id;
        Vec::new()
    }

    /// Gets historical associations for a concept.
    ///
    /// Returns associated concepts based on historical association type:
    /// - SAME_AS (900000000000527005)
    /// - REPLACED_BY (900000000000526001)
    /// - POSSIBLY_EQUIVALENT_TO (900000000000523009)
    /// - etc.
    fn get_historical_associations_by_type(
        &self,
        concept_id: SctId,
        association_type: HistoryAssociationType,
    ) -> Vec<SctId> {
        let _ = (concept_id, association_type);
        Vec::new()
    }

    /// Resolves an alternate identifier to a SNOMED CT concept ID.
    ///
    /// Alternate identifiers use different schemes to identify concepts:
    /// - `http://snomed.info/id/73211009` - SNOMED CT URI scheme
    /// - `http://snomed.info/sct#73211009` - SNOMED CT fragment scheme
    /// - Other custom identifier schemes
    ///
    /// Returns `Some(concept_id)` if the identifier can be resolved, `None` otherwise.
    ///
    /// # Arguments
    /// * `scheme` - The identifier scheme (URI prefix)
    /// * `identifier` - The identifier value within that scheme
    fn resolve_alternate_identifier(&self, scheme: &str, identifier: &str) -> Option<SctId> {
        // Default implementation: try to parse SNOMED CT URIs
        // http://snomed.info/id/73211009 or http://snomed.info/sct#73211009
        if scheme == "http://snomed.info/id" || scheme == "http://snomed.info/sct" {
            identifier.parse::<SctId>().ok().filter(|&id| self.has_concept(id))
        } else {
            // Unknown scheme - can be overridden by implementations
            None
        }
    }
}

/// Historical association types for history supplements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryAssociationType {
    /// SAME_AS association (900000000000527005).
    SameAs,
    /// REPLACED_BY association (900000000000526001).
    ReplacedBy,
    /// POSSIBLY_EQUIVALENT_TO association (900000000000523009).
    PossiblyEquivalentTo,
    /// ALTERNATIVE association (900000000000530003).
    Alternative,
    /// WAS_A association (900000000000528000).
    WasA,
    /// MOVED_TO association (900000000000524003).
    MovedTo,
    /// MOVED_FROM association (900000000000525002).
    MovedFrom,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock store for testing EclQueryable trait.
    struct MockStore {
        concepts: std::collections::HashSet<SctId>,
        children: std::collections::HashMap<SctId, Vec<SctId>>,
        parents: std::collections::HashMap<SctId, Vec<SctId>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                concepts: std::collections::HashSet::new(),
                children: std::collections::HashMap::new(),
                parents: std::collections::HashMap::new(),
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

    #[test]
    fn test_mock_store_has_concept() {
        let mut store = MockStore::new();
        store.add_concept(100);
        store.add_concept(200);

        assert!(store.has_concept(100));
        assert!(store.has_concept(200));
        assert!(!store.has_concept(300));
    }

    #[test]
    fn test_mock_store_hierarchy() {
        let mut store = MockStore::new();
        store.add_concept(100);
        store.add_concept(200);
        store.add_concept(300);

        // 200 IS_A 100
        // 300 IS_A 100
        store.add_is_a(200, 100);
        store.add_is_a(300, 100);

        let children = store.get_children(100);
        assert_eq!(children.len(), 2);
        assert!(children.contains(&200));
        assert!(children.contains(&300));

        let parents_200 = store.get_parents(200);
        assert_eq!(parents_200, vec![100]);

        let parents_100 = store.get_parents(100);
        assert!(parents_100.is_empty());
    }

    #[test]
    fn test_mock_store_all_concept_ids() {
        let mut store = MockStore::new();
        store.add_concept(100);
        store.add_concept(200);
        store.add_concept(300);

        let ids: std::collections::HashSet<SctId> = store.all_concept_ids().collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&100));
        assert!(ids.contains(&200));
        assert!(ids.contains(&300));
    }
}
