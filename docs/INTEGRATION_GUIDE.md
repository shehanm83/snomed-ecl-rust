# Integration Guide

This guide explains how to integrate `snomed-ecl-executor` with any SNOMED CT store.

## Overview

The `snomed-ecl-executor` crate is designed to be **store-agnostic**. It defines the
`EclQueryable` trait that you implement for your specific SNOMED CT data store.

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Application                          │
│                                                              │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   Your Store    │    │  snomed-ecl-executor            │ │
│  │  (SNOMED data)  │◄───│  (ECL query engine)             │ │
│  │                 │    │                                 │ │
│  │ impl EclQueryable    │  EclExecutor::new(&store)       │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Step 1: Add Dependencies

```toml
[dependencies]
snomed-ecl-executor = { git = "https://github.com/shehanm83/snomed-ecl-rust.git" }
```

Or if using a local path:

```toml
[dependencies]
snomed-ecl-executor = { path = "../snomed-ecl-rust/crates/snomed-ecl-executor" }
```

## Step 2: Implement EclQueryable Trait

The `EclQueryable` trait requires 5 methods to be implemented:

```rust
use snomed_ecl_executor::{EclQueryable, SctId};

pub struct MyStore {
    // Your data structures for concepts, relationships, etc.
}

impl EclQueryable for MyStore {
    /// Get direct children (concepts that have IS_A relationship TO this concept)
    fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
        // Return IDs of concepts where:
        //   child --[IS_A]--> concept_id
        todo!()
    }

    /// Get direct parents (concepts that this concept has IS_A relationship TO)
    fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
        // Return IDs of concepts where:
        //   concept_id --[IS_A]--> parent
        todo!()
    }

    /// Check if concept exists in the store
    fn has_concept(&self, concept_id: SctId) -> bool {
        todo!()
    }

    /// Return iterator over ALL concept IDs (used for * wildcard queries)
    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> {
        todo!()
    }

    /// Get members of a reference set (used for ^ member-of queries)
    fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId> {
        // Return empty Vec if reference sets not supported
        Vec::new()
    }
}
```

### Example: HashMap-based Store

```rust
use std::collections::{HashMap, HashSet};
use snomed_ecl_executor::{EclQueryable, SctId};

pub struct SimpleStore {
    concepts: HashSet<SctId>,
    /// Map from parent -> children
    children_map: HashMap<SctId, Vec<SctId>>,
    /// Map from child -> parents
    parents_map: HashMap<SctId, Vec<SctId>>,
}

impl SimpleStore {
    pub fn new() -> Self {
        Self {
            concepts: HashSet::new(),
            children_map: HashMap::new(),
            parents_map: HashMap::new(),
        }
    }

    pub fn add_concept(&mut self, id: SctId) {
        self.concepts.insert(id);
    }

    /// Add IS_A relationship: child IS_A parent
    pub fn add_is_a(&mut self, child: SctId, parent: SctId) {
        self.children_map.entry(parent).or_default().push(child);
        self.parents_map.entry(child).or_default().push(parent);
    }
}

impl EclQueryable for SimpleStore {
    fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
        self.children_map.get(&concept_id).cloned().unwrap_or_default()
    }

    fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
        self.parents_map.get(&concept_id).cloned().unwrap_or_default()
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
```

## Step 3: Use the Executor

```rust
use snomed_ecl_executor::EclExecutor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and populate your store
    let mut store = SimpleStore::new();

    // Add concepts (example: mini diabetes hierarchy)
    let diabetes = 73211009;
    let type1_diabetes = 46635009;
    let type2_diabetes = 44054006;

    store.add_concept(diabetes);
    store.add_concept(type1_diabetes);
    store.add_concept(type2_diabetes);

    // Type 1 IS_A Diabetes, Type 2 IS_A Diabetes
    store.add_is_a(type1_diabetes, diabetes);
    store.add_is_a(type2_diabetes, diabetes);

    // Create executor
    let executor = EclExecutor::new(&store);

    // Execute ECL queries
    let descendants = executor.execute("<< 73211009")?;
    println!("Diabetes and subtypes: {:?}", descendants.to_vec());

    // Check if a concept matches a constraint
    let is_diabetes_type = executor.matches(type1_diabetes, "<< 73211009")?;
    println!("Type 1 is a diabetes type: {}", is_diabetes_type);

    Ok(())
}
```

## Optional: Advanced EclQueryable Methods

For advanced ECL features (refinements, filters), implement these optional methods:

```rust
impl EclQueryable for MyStore {
    // ... required methods ...

    /// Get attribute relationships (non-IS_A relationships)
    /// Used for: < 404684003 : 363698007 = << 39057004
    fn get_attributes(&self, concept_id: SctId) -> Vec<RelationshipInfo> {
        // Return relationships like Finding site, Causative agent, etc.
        Vec::new()
    }

    /// Get concepts with specific attribute value
    /// Used for dot notation: < 404684003 . 363698007
    fn get_concepts_with_attribute(
        &self,
        attribute_type_id: SctId,
        target_id: SctId
    ) -> Vec<SctId> {
        Vec::new()
    }

    /// Get descriptions for term filtering
    /// Used for: < 404684003 {{ term = "heart" }}
    fn get_descriptions(&self, concept_id: SctId) -> Vec<DescriptionInfo> {
        Vec::new()
    }

    /// Check if concept is active
    /// Used for: < 404684003 {{ active = true }}
    fn is_concept_active(&self, concept_id: SctId) -> bool {
        self.has_concept(concept_id)
    }
}
```

## Configuration

```rust
use snomed_ecl_executor::{EclExecutor, ExecutorConfig, CacheConfig};
use std::time::Duration;

let config = ExecutorConfig::builder()
    .with_cache(CacheConfig {
        max_entries: 10_000,           // Cache up to 10k queries
        ttl: Duration::from_secs(300), // 5-minute TTL
        cache_intermediates: true,     // Cache sub-query results
    })
    .with_parallel(true)               // Enable parallel traversal
    .with_max_results(100_000)         // Limit result set size
    .build();

let executor = EclExecutor::with_config(&store, config);
```

## Supported ECL Features

| Operator | Example | Description |
|----------|---------|-------------|
| Self | `73211009` | Single concept |
| Descendant of | `< 73211009` | All descendants |
| Descendant or self | `<< 73211009` | Self + all descendants |
| Ancestor of | `> 73211009` | All ancestors |
| Ancestor or self | `>> 73211009` | Self + all ancestors |
| Child of | `<! 73211009` | Direct children only |
| Parent of | `>! 73211009` | Direct parents only |
| AND | `<< A AND << B` | Intersection |
| OR | `<< A OR << B` | Union |
| MINUS | `<< A MINUS << B` | Difference |
| Member of | `^ 700043003` | Reference set members |
| Any | `*` | All concepts |

## Thread Safety

`EclQueryable` requires `Send + Sync`, making it safe to share stores across threads:

```rust
use std::sync::Arc;

let store = Arc::new(MyStore::new());

// Clone Arc for each thread
let store_clone = Arc::clone(&store);
std::thread::spawn(move || {
    let executor = EclExecutor::new(store_clone.as_ref());
    executor.execute("<< 73211009").unwrap();
});
```

## Performance Tips

1. **Cache hierarchy lookups** - `get_children` and `get_parents` are called frequently
2. **Use indexes** - Hash maps for O(1) lookups
3. **Enable parallel feature** - For large traversals
4. **Pre-compute common queries** - Cache executor results for frequent ECL patterns

## Example: Integration with gRPC Service

```rust
use tonic::{Request, Response, Status};

pub struct EclService {
    store: Arc<MyStore>,
}

impl EclService {
    pub async fn execute_ecl(
        &self,
        request: Request<EclRequest>,
    ) -> Result<Response<EclResponse>, Status> {
        let ecl = request.into_inner().ecl;

        let executor = EclExecutor::new(self.store.as_ref());

        let result = executor.execute(&ecl)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        Ok(Response::new(EclResponse {
            concept_ids: result.to_vec(),
            count: result.count() as u64,
        }))
    }
}
```
