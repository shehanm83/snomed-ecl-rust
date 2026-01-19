# EclQueryable Trait Reference

Complete reference for implementing the `EclQueryable` trait to integrate with the ECL executor.

## Overview

The `EclQueryable` trait is the bridge between the ECL executor and your SNOMED CT data store. By implementing this trait, you enable the executor to query your data regardless of how it's stored.

```rust
pub trait EclQueryable: Send + Sync {
    // Required methods (5)
    fn get_children(&self, concept_id: SctId) -> Vec<SctId>;
    fn get_parents(&self, concept_id: SctId) -> Vec<SctId>;
    fn has_concept(&self, concept_id: SctId) -> bool;
    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_>;
    fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId>;

    // Optional methods (with defaults) - for advanced features
    fn get_attributes(&self, concept_id: SctId) -> Vec<RelationshipInfo> { vec![] }
    fn get_descriptions(&self, concept_id: SctId) -> Vec<DescriptionInfo> { vec![] }
    fn is_concept_active(&self, concept_id: SctId) -> bool { self.has_concept(concept_id) }
    fn is_concept_primitive(&self, concept_id: SctId) -> Option<bool> { None }
    // ... and more
}
```

## Type Alias

```rust
pub type SctId = u64;  // SNOMED CT Identifier (18 digits max)
```

---

## Required Methods

These 5 methods MUST be implemented. They provide the core functionality for hierarchy traversal and basic queries.

### get_children

```rust
fn get_children(&self, concept_id: SctId) -> Vec<SctId>;
```

Returns the **direct children** of a concept (one level down in the hierarchy).

**Hierarchy Direction:**
```
Parent Concept (concept_id)
    │
    ├── Child 1  ──┐
    ├── Child 2  ──┼── These are returned
    └── Child 3  ──┘
```

**Used For:**
- `<! concept` (child of)
- `< concept` (descendant of - recursive)
- `<< concept` (descendant or self - recursive)

**Example Implementation:**
```rust
fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
    self.children_map
        .get(&concept_id)
        .cloned()
        .unwrap_or_default()
}
```

**Performance Note:** This is called frequently during descendant traversal. Use indexed lookups (HashMap) for O(1) access.

---

### get_parents

```rust
fn get_parents(&self, concept_id: SctId) -> Vec<SctId>;
```

Returns the **direct parents** of a concept (one level up in the hierarchy).

**Hierarchy Direction:**
```
    ┌── Parent 1  ──┐
    ├── Parent 2  ──┼── These are returned
    └── Parent 3  ──┘
         │
    Child Concept (concept_id)
```

**Used For:**
- `>! concept` (parent of)
- `> concept` (ancestor of - recursive)
- `>> concept` (ancestor or self - recursive)

**Note:** SNOMED CT concepts can have multiple parents (polyhierarchy). A concept like "Viral pneumonia" might have parents "Viral disease" AND "Pneumonia".

**Example Implementation:**
```rust
fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
    self.parents_map
        .get(&concept_id)
        .cloned()
        .unwrap_or_default()
}
```

---

### has_concept

```rust
fn has_concept(&self, concept_id: SctId) -> bool;
```

Checks if a concept exists in the store.

**Used For:**
- Validating concept references in ECL
- Self constraints (`73211009`)
- Error detection (ConceptNotFound)

**Example Implementation:**
```rust
fn has_concept(&self, concept_id: SctId) -> bool {
    self.concepts.contains(&concept_id)
}
```

---

### all_concept_ids

```rust
fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_>;
```

Returns an iterator over ALL concept IDs in the store.

**Used For:**
- Wildcard queries (`*`)
- Finding all concepts with certain attributes

**Example Implementation:**
```rust
fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> {
    Box::new(self.concepts.iter().copied())
}
```

**Performance Note:** Wildcard queries can be expensive on large stores (350k+ concepts). Consider caching or limiting results.

---

### get_refset_members

```rust
fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId>;
```

Returns all concepts that are members of a reference set.

**Used For:**
- Member of queries (`^ refsetId`)
- Enhanced member of (`^ (expression)`)

**Example Implementation:**
```rust
fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId> {
    self.refset_members
        .get(&refset_id)
        .cloned()
        .unwrap_or_default()
}
```

**Note:** Return empty Vec if reference sets are not supported. This won't cause errors, but `^` queries will return empty results.

---

## Optional Methods - Attributes

These methods enable attribute refinement queries like `< 404684003 : 363698007 = << 39057004`.

### get_attributes

```rust
fn get_attributes(&self, concept_id: SctId) -> Vec<RelationshipInfo>;
```

Returns all non-IS_A relationships where this concept is the **source**.

**RelationshipInfo Structure:**
```rust
pub struct RelationshipInfo {
    pub type_id: SctId,        // Attribute type (e.g., 363698007 = Finding site)
    pub destination_id: SctId, // Target concept
    pub group: u16,            // Relationship group (0 = ungrouped)
}
```

**Used For:**
- Attribute refinements (`: attribute = value`)
- Attribute groups (`{ attr1, attr2 }`)
- Cardinality constraints

**Example:**
```
Diabetes mellitus (73211009)
  ├── Finding site (363698007) = Pancreatic structure (15776009)  [group 0]
  └── Pathological process (370135005) = ... [group 1]
```

**Example Implementation:**
```rust
fn get_attributes(&self, concept_id: SctId) -> Vec<RelationshipInfo> {
    self.relationships
        .iter()
        .filter(|r| r.source_id == concept_id && r.type_id != IS_A_ID)
        .map(|r| RelationshipInfo {
            type_id: r.type_id,
            destination_id: r.destination_id,
            group: r.relationship_group,
        })
        .collect()
}
```

---

### get_inbound_relationships

```rust
fn get_inbound_relationships(&self, concept_id: SctId) -> Vec<RelationshipInfo>;
```

Returns relationships where this concept is the **destination** (target).

**Used For:**
- Reverse attribute queries (`R` flag)
- "What has this as a finding site?"

**Example:**
```
Heart structure (80891009)
  ◄── Finding site from: Heart disease (56265001)
  ◄── Finding site from: Cardiac arrest (410429000)
```

---

### get_concepts_with_attribute

```rust
fn get_concepts_with_attribute(
    &self,
    attribute_type_id: SctId,
    target_id: SctId
) -> Vec<SctId>;
```

Returns concepts that have a specific attribute with a specific value.

**Used For:**
- Dot notation (`. attribute`)
- Reverse lookups

---

### get_concrete_values

```rust
fn get_concrete_values(&self, concept_id: SctId) -> Vec<ConcreteRelationshipInfo>;
```

Returns concrete domain values (numbers, strings) for a concept.

**ConcreteRelationshipInfo:**
```rust
pub struct ConcreteRelationshipInfo {
    pub type_id: SctId,
    pub value: ConcreteValueRef,
    pub group: u16,
}

pub enum ConcreteValueRef {
    Integer(i64),
    Decimal(f64),
    String(String),
}
```

**Used For:**
- Concrete value refinements (`#500`, `#3.14`, `#"text"`)
- Numeric comparisons (`>= #100`)

---

## Optional Methods - Filters

These methods enable filter constraints like `{{ active = true }}`.

### is_concept_active

```rust
fn is_concept_active(&self, concept_id: SctId) -> bool;
```

Returns whether a concept is active (not retired/inactive).

**Default:** Returns `has_concept(concept_id)` (assumes all existing concepts are active)

**Used For:**
- `{{ active = true }}` / `{{ active = false }}`

---

### is_concept_primitive

```rust
fn is_concept_primitive(&self, concept_id: SctId) -> Option<bool>;
```

Returns whether a concept is primitive (vs fully defined).

**Returns:**
- `Some(true)` - Primitive
- `Some(false)` - Fully Defined
- `None` - Unknown/not found

**Used For:**
- `{{ definitionStatus = primitive }}`
- `{{ definitionStatus = defined }}`

---

### get_concept_module

```rust
fn get_concept_module(&self, concept_id: SctId) -> Option<SctId>;
```

Returns the module ID for a concept.

**Used For:**
- `{{ moduleId = 900000000000207008 }}`

**Common Modules:**
| ID | Name |
|----|------|
| 900000000000207008 | SNOMED CT core |
| 731000124108 | US Extension |

---

### get_concept_effective_time

```rust
fn get_concept_effective_time(&self, concept_id: SctId) -> Option<u32>;
```

Returns the effective time (release date) as YYYYMMDD.

**Used For:**
- `{{ effectiveTime >= 20200101 }}`

**Format:** `20200131` for January 31, 2020

---

### get_descriptions

```rust
fn get_descriptions(&self, concept_id: SctId) -> Vec<DescriptionInfo>;
```

Returns all descriptions (terms) for a concept.

**DescriptionInfo Structure:**
```rust
pub struct DescriptionInfo {
    pub description_id: SctId,
    pub term: String,
    pub language_code: String,        // "en", "es", etc.
    pub type_id: SctId,               // FSN, Synonym, Definition
    pub case_significance_id: SctId,
    pub active: bool,
    pub effective_time: Option<u32>,
    pub module_id: SctId,
}
```

**Used For:**
- `{{ term = "diabetes" }}`
- `{{ language = en }}`
- `{{ type = fsn }}`

**Description Type IDs:**
| ID | Type |
|----|------|
| 900000000000003001 | Fully Specified Name |
| 900000000000013009 | Synonym |
| 900000000000550004 | Definition |

---

### get_semantic_tag

```rust
fn get_semantic_tag(&self, concept_id: SctId) -> Option<String>;
```

Returns the semantic tag from the FSN.

**Example:** "Diabetes mellitus (disorder)" → `"disorder"`

**Used For:**
- `{{ semanticTag = "disorder" }}`

**Default Implementation:** Extracts from FSN if `get_descriptions` is implemented.

---

### get_preferred_term

```rust
fn get_preferred_term(&self, concept_id: SctId) -> Option<String>;
```

Returns the preferred term for display.

---

### get_description_language_refsets

```rust
fn get_description_language_refsets(&self, description_id: SctId) -> Vec<LanguageRefsetMember>;
```

Returns language reference set memberships for a description.

**LanguageRefsetMember:**
```rust
pub struct LanguageRefsetMember {
    pub refset_id: SctId,           // Language refset ID
    pub acceptability: Acceptability, // Preferred or Acceptable
}

pub enum Acceptability {
    Preferred,
    Acceptable,
}
```

**Used For:**
- `{{ dialect = en-US (preferred) }}`
- `{{ preferredIn = 900000000000509007 }}`

---

## Optional Methods - History

### get_historical_associations

```rust
fn get_historical_associations(&self, concept_id: SctId) -> Vec<SctId>;
```

Returns concepts that replaced this (inactive) concept.

**Used For:**
- `{{ +HISTORY }}`

---

### get_historical_associations_by_type

```rust
fn get_historical_associations_by_type(
    &self,
    concept_id: SctId,
    association_type: HistoryAssociationType,
) -> Vec<SctId>;
```

Returns specific types of historical associations.

**HistoryAssociationType:**
```rust
pub enum HistoryAssociationType {
    SameAs,              // Exact replacement
    ReplacedBy,          // Replaced by another concept
    PossiblyEquivalentTo,
    Alternative,
    WasA,
    MovedTo,
    MovedFrom,
}
```

**Used For:**
- `{{ +HISTORY-MIN }}` (SameAs only)
- `{{ +HISTORY-MOD }}` (SameAs + ReplacedBy)
- `{{ +HISTORY-MAX }}` (All types)

---

## Optional Methods - Advanced

### resolve_alternate_identifier

```rust
fn resolve_alternate_identifier(&self, scheme: &str, identifier: &str) -> Option<SctId>;
```

Resolves URI-based identifiers to concept IDs.

**Used For:**
- `http://snomed.info/id/73211009`
- `http://snomed.info/sct#73211009`

**Default Implementation:** Handles standard SNOMED URIs automatically.

---

## Complete Implementation Example

```rust
use std::collections::{HashMap, HashSet};
use snomed_ecl_executor::{
    EclQueryable, SctId, RelationshipInfo, DescriptionInfo, ConceptInfo
};

pub struct SnomedStore {
    concepts: HashSet<SctId>,
    children: HashMap<SctId, Vec<SctId>>,
    parents: HashMap<SctId, Vec<SctId>>,
    relationships: HashMap<SctId, Vec<RelationshipInfo>>,
    refset_members: HashMap<SctId, Vec<SctId>>,
    descriptions: HashMap<SctId, Vec<DescriptionInfo>>,
    concept_info: HashMap<SctId, ConceptInfo>,
}

impl EclQueryable for SnomedStore {
    // ===== Required Methods =====

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

    fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId> {
        self.refset_members.get(&refset_id).cloned().unwrap_or_default()
    }

    // ===== Optional Methods =====

    fn get_attributes(&self, concept_id: SctId) -> Vec<RelationshipInfo> {
        self.relationships.get(&concept_id).cloned().unwrap_or_default()
    }

    fn get_descriptions(&self, concept_id: SctId) -> Vec<DescriptionInfo> {
        self.descriptions.get(&concept_id).cloned().unwrap_or_default()
    }

    fn is_concept_active(&self, concept_id: SctId) -> bool {
        self.concept_info
            .get(&concept_id)
            .map(|info| info.active)
            .unwrap_or(false)
    }

    fn is_concept_primitive(&self, concept_id: SctId) -> Option<bool> {
        self.concept_info.get(&concept_id).map(|info| info.is_primitive)
    }

    fn get_concept_module(&self, concept_id: SctId) -> Option<SctId> {
        self.concept_info.get(&concept_id).map(|info| info.module_id)
    }

    fn get_concept_effective_time(&self, concept_id: SctId) -> Option<u32> {
        self.concept_info.get(&concept_id).and_then(|info| info.effective_time)
    }
}
```

---

## Thread Safety

The trait requires `Send + Sync`, enabling safe concurrent access:

```rust
use std::sync::Arc;

let store = Arc::new(SnomedStore::new());

// Safe to share across threads
let handles: Vec<_> = (0..4)
    .map(|_| {
        let store = Arc::clone(&store);
        std::thread::spawn(move || {
            let executor = EclExecutor::new(store.as_ref());
            executor.execute("<< 73211009").unwrap()
        })
    })
    .collect();
```

---

## Performance Recommendations

1. **Use HashMaps** for O(1) lookups in `get_children`, `get_parents`, `has_concept`
2. **Index relationships** by source concept for `get_attributes`
3. **Cache hierarchy** if building from database on startup
4. **Lazy load** descriptions and concrete values (called less frequently)
5. **Consider read-write locks** for concurrent access to mutable stores

---

## Next Steps

- See [USAGE.md](USAGE.md) for complete usage examples
- See [../optimizer/README.md](../optimizer/README.md) for performance optimizations
