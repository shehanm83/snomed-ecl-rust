# snomed-ecl-executor

High-performance ECL execution engine for SNOMED CT.

## Overview

The `snomed-ecl-executor` crate provides a complete query execution engine for ECL expressions. While the `snomed-ecl` parser converts ECL strings to an AST, this crate actually **evaluates** those expressions against your SNOMED CT data.

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Your Application                              │
│                                                                       │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │  ECL String  │───▶│  snomed-ecl      │───▶│  AST             │   │
│  │              │    │  (parser)        │    │                  │   │
│  └──────────────┘    └──────────────────┘    └────────┬─────────┘   │
│                                                        │             │
│                                                        ▼             │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │  Your Store  │◀───│  snomed-ecl-     │◀───│  EclExecutor     │   │
│  │  (SNOMED CT) │    │  executor        │    │                  │   │
│  └──────────────┘    └──────────────────┘    └──────────────────┘   │
│        │                      │                       │             │
│        │ impl EclQueryable    │                       │             │
│        ▼                      ▼                       ▼             │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                     Query Results                              │   │
│  │                  (Set of Concept IDs)                          │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Features

### Store-Agnostic Design

The executor works with **any** SNOMED CT data store through the `EclQueryable` trait:

- In-memory HashMaps
- PostgreSQL/MySQL databases
- Graph databases (Neo4j)
- Elasticsearch indices
- Custom data structures

### High Performance

- **LRU caching** of query results
- **Efficient traversal** algorithms for hierarchy queries
- **Configurable timeouts** to prevent runaway queries
- **Optional parallelism** for large traversals

### Complete ECL Support

Supports all ECL 2.2 features including:
- All hierarchy operators
- Compound expressions (AND, OR, MINUS)
- Attribute refinements with groups
- All filter types
- Concrete values
- History supplements

## Quick Start

```rust
use snomed_ecl_executor::{EclExecutor, EclQueryable, SctId};

// 1. Implement EclQueryable for your store
struct MyStore { /* your SNOMED CT data */ }

impl EclQueryable for MyStore {
    fn get_children(&self, id: SctId) -> Vec<SctId> { todo!() }
    fn get_parents(&self, id: SctId) -> Vec<SctId> { todo!() }
    fn has_concept(&self, id: SctId) -> bool { todo!() }
    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> { todo!() }
    fn get_refset_members(&self, id: SctId) -> Vec<SctId> { vec![] }
}

// 2. Create executor and run queries
let store = MyStore { /* ... */ };
let executor = EclExecutor::new(&store);

// Execute ECL query
let result = executor.execute("<< 73211009 |Diabetes mellitus|")?;
println!("Found {} concepts", result.count());

// Check if concept matches constraint
let matches = executor.matches(46635009, "<< 73211009")?;
println!("Type 1 diabetes is a diabetes: {}", matches);
```

## Architecture

### Components

```
snomed-ecl-executor/
├── EclExecutor         # Main query executor
├── EclQueryable        # Trait for data stores
├── QueryResult         # Query result with stats
├── QueryPlanner        # Query optimization
├── HierarchyTraverser  # Efficient hierarchy traversal
├── QueryCache          # LRU result cache
└── ExecutorConfig      # Configuration options
```

### Query Execution Flow

1. **Parse** - ECL string → AST (uses snomed-ecl)
2. **Plan** - Analyze AST, estimate costs
3. **Execute** - Traverse hierarchy, apply constraints
4. **Filter** - Apply post-filters (active, term, etc.)
5. **Cache** - Store result for reuse
6. **Return** - QueryResult with concept IDs and stats

### Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Self constraint | O(1) | Single concept lookup |
| Child/Parent | O(k) | k = number of children/parents |
| Descendant/Ancestor | O(n) | n = tree size, cached |
| AND | O(min(a,b)) | Set intersection |
| OR | O(a+b) | Set union |
| MINUS | O(a) | Set difference |
| Wildcard | O(N) | N = total concepts |

## Configuration

```rust
use snomed_ecl_executor::{EclExecutor, ExecutorConfig, CacheConfig};
use std::time::Duration;

let config = ExecutorConfig::builder()
    .with_cache(CacheConfig {
        max_entries: 10_000,           // Cache up to 10k queries
        ttl: Duration::from_secs(300), // 5-minute TTL
        cache_intermediates: true,     // Cache sub-expression results
    })
    .with_parallel(true)               // Enable parallel traversal
    .with_max_results(100_000)         // Limit result set size
    .build();

let executor = EclExecutor::with_config(&store, config);
```

## Documentation

- **[TRAIT.md](TRAIT.md)** - Complete EclQueryable trait reference
- **[USAGE.md](USAGE.md)** - Detailed usage examples and patterns

## Thread Safety

The executor is thread-safe when your `EclQueryable` implementation is thread-safe:

```rust
use std::sync::Arc;

let store = Arc::new(MyStore::new());
let executor = EclExecutor::new(store.as_ref());

// Share across threads
let executor_clone = executor.clone();
std::thread::spawn(move || {
    executor_clone.execute("<< 73211009").unwrap();
});
```

## Error Handling

```rust
use snomed_ecl_executor::{EclExecutor, EclExecutorError};

match executor.execute(ecl) {
    Ok(result) => println!("Found {} concepts", result.count()),
    Err(EclExecutorError::ParseError(e)) => eprintln!("Invalid ECL: {}", e),
    Err(EclExecutorError::ConceptNotFound(id)) => eprintln!("Concept {} not found", id),
    Err(EclExecutorError::Timeout) => eprintln!("Query timed out"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Installation

```toml
[dependencies]
snomed-ecl-executor = { git = "https://github.com/your-repo/snomed-ecl-rust.git" }
```

## Next Steps

1. Read [TRAIT.md](TRAIT.md) to understand the EclQueryable trait
2. Read [USAGE.md](USAGE.md) for detailed examples
3. See [../optimizer/README.md](../optimizer/README.md) for performance optimizations
