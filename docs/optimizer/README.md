# snomed-ecl-optimizer

Performance optimizations for ECL query execution on large SNOMED CT datasets.

## Overview

The `snomed-ecl-optimizer` crate provides optional performance enhancements for the ECL executor. While the base executor works well for small to medium datasets, large-scale deployments (full SNOMED CT with 350k+ concepts) benefit from these optimizations.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Performance Comparison                           │
├─────────────────────────────────────────────────────────────────────────┤
│  Query: << 404684003 (Clinical finding)                                  │
│  Dataset: Full SNOMED CT (~350,000 concepts)                             │
│                                                                          │
│  Base Executor:      ~500ms  (tree traversal)                           │
│  With Closure:       ~1ms    (precomputed lookup)                       │
│  With Bitset:        ~0.1ms  (bitmap operations)                        │
└─────────────────────────────────────────────────────────────────────────┘
```

## Features

| Feature | Description | Use Case |
|---------|-------------|----------|
| `closure` | Precomputed transitive closure | O(1) ancestor/descendant queries |
| `bitset` | Roaring bitmap concept sets | Fast set operations |
| `persistence` | Save/load optimized structures | Faster startup |
| `filter-service` | Runtime filter caching | Repeated filter queries |
| `full` | All optimizations | Production deployments |

## Installation

```toml
[dependencies]
snomed-ecl-optimizer = {
    git = "https://github.com/your-repo/snomed-ecl-rust.git",
    features = ["full"]  # Or specific features
}
```

---

## Transitive Closure

### What is Transitive Closure?

The transitive closure precomputes ALL ancestor-descendant relationships, turning O(n) tree traversal into O(1) lookup.

**Without closure:**
```
Is 46635009 a descendant of 404684003?
→ Walk up tree: 46635009 → 73211009 → 64572001 → 404684003 → ...
→ O(depth) per query
```

**With closure:**
```
Is 46635009 a descendant of 404684003?
→ Lookup: descendants[404684003].contains(46635009)
→ O(1) per query
```

### Building the Closure

```rust
use snomed_ecl_optimizer::closure::TransitiveClosure;
use snomed_ecl_executor::EclExecutor;

// Build closure from your store (one-time cost)
let closure = TransitiveClosure::build(&store);

// Print statistics
println!("Closure built:");
println!("  Concepts: {}", closure.concept_count());
println!("  Relationships: {}", closure.relationship_count());
println!("  Memory: {} MB", closure.memory_usage() / 1024 / 1024);

// Use closure as the query store
let executor = EclExecutor::new(&closure);

// Now hierarchy queries are O(1)
let result = executor.execute("<< 404684003")?;  // Instant!
```

### Closure Memory Usage

The closure uses memory proportional to the number of relationships:

| Dataset | Concepts | Relationships | Memory |
|---------|----------|---------------|--------|
| Mini (test) | 1,000 | 5,000 | ~1 MB |
| Medium | 50,000 | 200,000 | ~50 MB |
| Full SNOMED | 350,000 | 1,500,000 | ~500 MB |

### Closure API

```rust
impl TransitiveClosure {
    /// Build from any EclQueryable store
    pub fn build<T: EclQueryable>(store: &T) -> Self;

    /// Check if ancestor-descendant relationship exists
    pub fn is_descendant_of(&self, concept: SctId, ancestor: SctId) -> bool;

    /// Get all descendants (precomputed)
    pub fn get_descendants(&self, concept: SctId) -> &HashSet<SctId>;

    /// Get all ancestors (precomputed)
    pub fn get_ancestors(&self, concept: SctId) -> &HashSet<SctId>;

    /// Memory usage in bytes
    pub fn memory_usage(&self) -> usize;
}
```

### Closure implements EclQueryable

```rust
// The closure itself implements EclQueryable
impl EclQueryable for TransitiveClosure {
    fn get_children(&self, id: SctId) -> Vec<SctId> { ... }
    fn get_parents(&self, id: SctId) -> Vec<SctId> { ... }
    fn has_concept(&self, id: SctId) -> bool { ... }
    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> { ... }
    fn get_refset_members(&self, id: SctId) -> Vec<SctId> { ... }

    // These are O(1) instead of O(n)!
    // (internally uses precomputed sets)
}
```

---

## Roaring Bitmaps

### What are Roaring Bitmaps?

Roaring bitmaps are compressed bitmap data structures optimized for set operations. They're much faster than HashSet for large concept sets.

**Performance comparison:**
```
Operation          HashSet      Roaring Bitmap
─────────────────────────────────────────────
Union (100k)       ~50ms        ~1ms
Intersection       ~30ms        ~0.5ms
Contains           O(1)         O(1)
Memory (100k)      ~3.2 MB      ~100 KB
```

### Using Bitmap Sets

```rust
use snomed_ecl_optimizer::bitset::ConceptBitset;

// Create bitset from concept IDs
let set1 = ConceptBitset::from_iter(descendants_of_diabetes);
let set2 = ConceptBitset::from_iter(descendants_of_clinical_finding);

// Fast set operations
let intersection = &set1 & &set2;  // AND
let union = &set1 | &set2;         // OR
let difference = &set1 - &set2;    // MINUS

// Check membership
if intersection.contains(46635009) {
    println!("Type 1 diabetes is in both sets");
}

// Iterate
for concept_id in intersection.iter() {
    println!("{}", concept_id);
}
```

### Bitmap-Optimized Executor

```rust
use snomed_ecl_optimizer::bitset::BitsetExecutor;

// Wrap executor with bitmap optimization
let bitset_executor = BitsetExecutor::new(&store);

// Compound queries use bitmap operations internally
let result = bitset_executor.execute(
    "<< 73211009 AND << 404684003 MINUS << 46635009"
)?;
```

---

## Persistence

### Saving Optimized Structures

```rust
use snomed_ecl_optimizer::persistence::{save_closure, load_closure};

// Build closure (slow, one-time)
let closure = TransitiveClosure::build(&store);

// Save to disk
save_closure(&closure, "closure.bin")?;

// Later, load from disk (fast)
let closure = load_closure("closure.bin")?;
```

### Binary Format

The persistence module uses a compact binary format:

```
┌──────────────────────────────────────────┐
│ Header (magic, version, counts)          │
├──────────────────────────────────────────┤
│ Concept IDs (sorted, delta-encoded)      │
├──────────────────────────────────────────┤
│ Descendant sets (compressed bitmaps)     │
├──────────────────────────────────────────┤
│ Ancestor sets (compressed bitmaps)       │
└──────────────────────────────────────────┘
```

**File sizes:**
| Dataset | In-Memory | On-Disk |
|---------|-----------|---------|
| Full SNOMED | ~500 MB | ~50 MB |

### Loading with Validation

```rust
use snomed_ecl_optimizer::persistence::{load_closure_validated, ClosureMetadata};

// Load with metadata check
let (closure, metadata) = load_closure_validated("closure.bin")?;

println!("Closure metadata:");
println!("  Created: {}", metadata.created_at);
println!("  SNOMED version: {}", metadata.snomed_version);
println!("  Concept count: {}", metadata.concept_count);

// Validate against current store
if metadata.concept_count != store.concept_count() {
    println!("Warning: closure may be stale");
}
```

---

## Filter Service

### Caching Filter Results

```rust
use snomed_ecl_optimizer::filter_service::FilterService;

// Create filter service with cache
let filter_service = FilterService::new(&store)
    .with_cache_size(10_000)
    .with_ttl(Duration::from_secs(300));

// Cached filter evaluation
let active_diabetes = filter_service.evaluate(
    "<< 73211009 {{ active = true }}"
)?;

// Second call uses cache
let active_diabetes_2 = filter_service.evaluate(
    "<< 73211009 {{ active = true }}"
)?;  // Cache hit!
```

### Preloading Common Filters

```rust
// Preload common queries at startup
filter_service.preload(&[
    "<< 404684003 {{ active = true }}",           // Active clinical findings
    "<< 373873005 {{ active = true }}",           // Active substances
    "<< 71388002 {{ active = true }}",            // Active procedures
]);
```

---

## Complete Production Setup

### Recommended Configuration

```rust
use snomed_ecl_optimizer::{
    closure::TransitiveClosure,
    bitset::BitsetExecutor,
    persistence::{load_closure, save_closure},
    filter_service::FilterService,
};
use snomed_ecl_executor::{EclExecutor, ExecutorConfig, CacheConfig};
use std::time::Duration;
use std::path::Path;

pub struct OptimizedEclEngine {
    closure: TransitiveClosure,
    executor: EclExecutor<'static, TransitiveClosure>,
    filter_service: FilterService,
}

impl OptimizedEclEngine {
    pub fn new(store: &impl EclQueryable, cache_path: &Path) -> Self {
        // Load or build closure
        let closure = if cache_path.exists() {
            println!("Loading closure from cache...");
            load_closure(cache_path).expect("Failed to load closure")
        } else {
            println!("Building closure (this may take a few minutes)...");
            let closure = TransitiveClosure::build(store);
            save_closure(&closure, cache_path).expect("Failed to save closure");
            closure
        };

        // Configure executor
        let config = ExecutorConfig::builder()
            .with_cache(CacheConfig {
                max_entries: 50_000,
                ttl: Duration::from_secs(600),
                cache_intermediates: true,
            })
            .with_max_results(500_000)
            .build();

        let executor = EclExecutor::with_config(&closure, config);

        // Create filter service
        let filter_service = FilterService::new(&closure)
            .with_cache_size(10_000)
            .with_ttl(Duration::from_secs(300));

        Self {
            closure,
            executor,
            filter_service,
        }
    }

    pub fn execute(&self, ecl: &str) -> Result<Vec<u64>, EclExecutorError> {
        self.executor.execute(ecl).map(|r| r.to_vec())
    }

    pub fn matches(&self, concept_id: u64, ecl: &str) -> Result<bool, EclExecutorError> {
        self.executor.matches(concept_id, ecl)
    }
}
```

### Startup Optimization

```rust
use std::thread;

fn initialize_engine(store: &MyStore) -> OptimizedEclEngine {
    let cache_path = Path::new("./cache/closure.bin");

    // Build closure in background during startup
    let engine = thread::spawn(move || {
        OptimizedEclEngine::new(store, cache_path)
    });

    // ... other initialization ...

    engine.join().unwrap()
}
```

### Memory Management

```rust
impl OptimizedEclEngine {
    /// Get memory usage statistics
    pub fn memory_stats(&self) -> MemoryStats {
        MemoryStats {
            closure_bytes: self.closure.memory_usage(),
            cache_entries: self.executor.cache_size(),
            filter_cache_entries: self.filter_service.cache_size(),
        }
    }

    /// Clear caches to free memory
    pub fn clear_caches(&self) {
        self.executor.clear_cache();
        self.filter_service.clear_cache();
    }
}
```

---

## Benchmarking

### Benchmark Script

```rust
use std::time::Instant;

fn benchmark_query(executor: &EclExecutor<impl EclQueryable>, ecl: &str, iterations: usize) {
    // Warm up
    let _ = executor.execute(ecl);

    // Benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = executor.execute(ecl);
    }
    let duration = start.elapsed();

    println!(
        "Query: {}\n  {} iterations in {:?}\n  Average: {:?}",
        ecl,
        iterations,
        duration,
        duration / iterations as u32
    );
}

fn main() {
    let store = load_snomed_data();

    // Benchmark without optimization
    let executor = EclExecutor::new(&store);
    benchmark_query(&executor, "<< 404684003", 100);

    // Benchmark with closure
    let closure = TransitiveClosure::build(&store);
    let optimized = EclExecutor::new(&closure);
    benchmark_query(&optimized, "<< 404684003", 100);
}
```

### Expected Results

| Query | Base | With Closure | Speedup |
|-------|------|--------------|---------|
| `<< 404684003` | 500ms | 1ms | 500x |
| `<< A AND << B` | 800ms | 2ms | 400x |
| `< A MINUS < B` | 600ms | 1.5ms | 400x |
| `> 46635009` | 100ms | 0.5ms | 200x |

---

## Feature Flags

### Selective Features

```toml
# Only transitive closure
snomed-ecl-optimizer = { ..., features = ["closure"] }

# Closure + persistence
snomed-ecl-optimizer = { ..., features = ["closure", "persistence"] }

# All features
snomed-ecl-optimizer = { ..., features = ["full"] }
```

### Feature Dependencies

```
full
├── closure
├── bitset
├── persistence (requires closure)
└── filter-service
```

---

## Troubleshooting

### Out of Memory

```rust
// Reduce closure memory by excluding rarely-used concepts
let closure = TransitiveClosure::builder()
    .exclude_inactive(true)      // Skip inactive concepts
    .max_depth(10)               // Limit traversal depth
    .build(&store);
```

### Slow Closure Building

```rust
// Build incrementally
let mut closure = TransitiveClosure::new();
for chunk in store.concept_ids().chunks(10_000) {
    closure.add_concepts(chunk, &store);
    println!("Progress: {} concepts", closure.concept_count());
}
```

### Stale Cache

```rust
// Check closure freshness
let metadata = load_closure_metadata("closure.bin")?;
let store_hash = compute_store_hash(&store);

if metadata.store_hash != store_hash {
    println!("Rebuilding stale closure...");
    let closure = TransitiveClosure::build(&store);
    save_closure(&closure, "closure.bin")?;
}
```

---

## Best Practices

1. **Build closure during deployment** - Not at runtime
2. **Persist to disk** - Avoid rebuilding on every restart
3. **Monitor memory** - Closure can use significant RAM
4. **Cache wisely** - Balance memory vs. query speed
5. **Benchmark your queries** - Optimization impact varies

---

## Next Steps

- See [../executor/README.md](../executor/README.md) for base executor usage
- See [../parser/README.md](../parser/README.md) for ECL syntax reference
