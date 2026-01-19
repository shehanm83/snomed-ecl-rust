# Executor Usage Guide

Comprehensive examples for using the `snomed-ecl-executor` crate.

## Installation

```toml
[dependencies]
snomed-ecl-executor = { git = "https://github.com/your-repo/snomed-ecl-rust.git" }
```

## Basic Usage

### Creating an Executor

```rust
use snomed_ecl_executor::{EclExecutor, EclQueryable};

// Assuming you have a store that implements EclQueryable
let store = MyStore::new();
let executor = EclExecutor::new(&store);
```

### Executing Queries

```rust
use snomed_ecl_executor::EclExecutor;

let executor = EclExecutor::new(&store);

// Execute an ECL query
let result = executor.execute("<< 73211009 |Diabetes mellitus|")?;

// Get result count
println!("Found {} concepts", result.count());

// Iterate over results
for concept_id in result.iter() {
    println!("Concept: {}", concept_id);
}

// Convert to Vec
let ids: Vec<u64> = result.to_vec();
```

### Checking Matches

```rust
// Check if a concept matches a constraint
let is_diabetes = executor.matches(46635009, "<< 73211009")?;
println!("Type 1 diabetes is a diabetes: {}", is_diabetes);

// Useful for validation
fn validate_diagnosis(executor: &EclExecutor<MyStore>, diagnosis_id: u64) -> bool {
    executor.matches(diagnosis_id, "<< 404684003 |Clinical finding|")
        .unwrap_or(false)
}
```

## Query Examples

### Hierarchy Queries

```rust
// All types of diabetes (not including diabetes itself)
let subtypes = executor.execute("< 73211009")?;

// Diabetes and all subtypes
let diabetes_all = executor.execute("<< 73211009")?;

// Direct children only
let direct_subtypes = executor.execute("<! 73211009")?;

// All ancestors of Type 1 diabetes
let ancestors = executor.execute("> 46635009")?;

// Direct parents only
let parents = executor.execute(">! 46635009")?;
```

### Compound Queries

```rust
// Intersection: concepts that are both A and B
let result = executor.execute("<< 73211009 AND << 64572001")?;

// Union: concepts that are A or B or both
let result = executor.execute("<< 73211009 OR << 38341003")?;

// Difference: A but not B
let result = executor.execute("<< 73211009 MINUS << 46635009")?;

// Complex compound
let result = executor.execute(
    "(<< 73211009 OR << 38341003) AND << 404684003"
)?;
```

### Reference Set Queries

```rust
// Members of a reference set
let members = executor.execute("^ 700043003")?;

// Members that are also descendants of clinical finding
let result = executor.execute("^ 700043003 AND << 404684003")?;
```

### Refinement Queries

```rust
// Concepts with specific finding site
let result = executor.execute(
    "< 404684003 : 363698007 = << 39057004"
)?;

// Multiple attributes
let result = executor.execute(
    "< 404684003 : 363698007 = << 39057004, 116676008 = << 49755003"
)?;

// Grouped attributes (must be in same group)
let result = executor.execute(
    "< 404684003 : { 363698007 = << 39057004, 116676008 = << 49755003 }"
)?;

// Any value (wildcard)
let result = executor.execute("< 404684003 : 363698007 = *")?;
```

### Filter Queries

```rust
// Active concepts only
let result = executor.execute("<< 73211009 {{ active = true }}")?;

// Primitive concepts
let result = executor.execute(
    "<< 73211009 {{ definitionStatus = primitive }}"
)?;

// Term filter
let result = executor.execute(
    r#"<< 404684003 {{ term = "heart" }}"#
)?;

// Multiple filters
let result = executor.execute(
    r#"<< 73211009 {{ active = true, definitionStatus = primitive }}"#
)?;

// Combined with refinement
let result = executor.execute(
    "<< 404684003 {{ active = true }} : 363698007 = *"
)?;
```

## Configuration

### Basic Configuration

```rust
use snomed_ecl_executor::{EclExecutor, ExecutorConfig, CacheConfig};
use std::time::Duration;

let config = ExecutorConfig::builder()
    .with_cache(CacheConfig {
        max_entries: 10_000,
        ttl: Duration::from_secs(300),
        cache_intermediates: true,
    })
    .build();

let executor = EclExecutor::with_config(&store, config);
```

### Cache Configuration

```rust
use snomed_ecl_executor::CacheConfig;
use std::time::Duration;

let cache_config = CacheConfig {
    // Maximum number of cached query results
    max_entries: 10_000,

    // Time-to-live for cache entries
    ttl: Duration::from_secs(300),

    // Cache intermediate results (sub-expressions)
    cache_intermediates: true,
};
```

### Performance Tuning

```rust
let config = ExecutorConfig::builder()
    // Limit maximum result size (prevents memory issues)
    .with_max_results(100_000)

    // Enable parallel traversal for large hierarchies
    .with_parallel(true)

    // Configure cache
    .with_cache(CacheConfig {
        max_entries: 50_000,
        ttl: Duration::from_secs(600),
        cache_intermediates: true,
    })

    .build();
```

## Working with Results

### QueryResult API

```rust
let result = executor.execute("<< 73211009")?;

// Count without iterating
let count = result.count();

// Check if empty
let is_empty = result.is_empty();

// Iterate
for id in result.iter() {
    println!("{}", id);
}

// Convert to Vec
let vec: Vec<u64> = result.to_vec();

// Into iterator (consumes result)
for id in result {
    println!("{}", id);
}
```

### Execution Statistics

```rust
let result = executor.execute("<< 73211009")?;

if let Some(stats) = result.stats() {
    println!("Execution time: {:?}", stats.execution_time);
    println!("Concepts evaluated: {}", stats.concepts_evaluated);
    println!("Cache hits: {}", stats.cache_hits);
}
```

## Convenience Methods

### Get Descendants

```rust
// Direct API (no ECL parsing)
let descendants = executor.get_descendants(73211009)?;

// With limit
let limited = executor.get_descendants_limited(73211009, 100)?;
```

### Get Ancestors

```rust
let ancestors = executor.get_ancestors(46635009)?;
```

### Get Children/Parents

```rust
let children = executor.get_children(73211009)?;
let parents = executor.get_parents(46635009)?;
```

### Subsumption Check

```rust
// Is concept A subsumed by concept B?
// i.e., Is A a descendant of B?
let is_subsumed = executor.is_subsumed_by(46635009, 73211009)?;
println!("Type 1 is subsumed by Diabetes: {}", is_subsumed);
```

## Query Planning

### Explain Query

```rust
// Get query plan without executing
let plan = executor.explain("<< 73211009 AND << 64572001")?;

println!("Query plan:");
for step in plan.steps() {
    println!("  - {}: estimated {} concepts", step.operation, step.estimated_count);
}
println!("Total estimated cost: {}", plan.total_cost());
```

## Error Handling

### Error Types

```rust
use snomed_ecl_executor::EclExecutorError;

match executor.execute(ecl) {
    Ok(result) => {
        println!("Success: {} concepts", result.count());
    }
    Err(EclExecutorError::ParseError(e)) => {
        // Invalid ECL syntax
        eprintln!("Parse error: {}", e);
    }
    Err(EclExecutorError::ConceptNotFound(id)) => {
        // Referenced concept doesn't exist
        eprintln!("Concept {} not found", id);
    }
    Err(EclExecutorError::RefsetNotFound(id)) => {
        // Referenced refset doesn't exist
        eprintln!("Refset {} not found", id);
    }
    Err(EclExecutorError::Timeout) => {
        // Query exceeded time limit
        eprintln!("Query timed out");
    }
    Err(EclExecutorError::ResultTooLarge(count)) => {
        // Result exceeded max_results
        eprintln!("Result too large: {} concepts", count);
    }
    Err(EclExecutorError::UnsupportedFeature(feature)) => {
        // ECL feature not supported
        eprintln!("Unsupported: {}", feature);
    }
}
```

### Safe Query Execution

```rust
fn safe_execute(executor: &EclExecutor<MyStore>, ecl: &str) -> Vec<u64> {
    executor.execute(ecl)
        .map(|r| r.to_vec())
        .unwrap_or_default()
}

fn validate_ecl(executor: &EclExecutor<MyStore>, ecl: &str) -> Result<(), String> {
    executor.execute(ecl)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

## Integration Patterns

### Web Service

```rust
use actix_web::{web, HttpResponse};

struct AppState {
    executor: EclExecutor<'static, MyStore>,
}

async fn execute_ecl(
    data: web::Data<AppState>,
    query: web::Query<EclQuery>,
) -> HttpResponse {
    match data.executor.execute(&query.ecl) {
        Ok(result) => HttpResponse::Ok().json(EclResponse {
            count: result.count(),
            concept_ids: result.to_vec(),
        }),
        Err(e) => HttpResponse::BadRequest().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}
```

### gRPC Service

```rust
use tonic::{Request, Response, Status};

pub struct EclService {
    executor: EclExecutor<'static, MyStore>,
}

#[tonic::async_trait]
impl EclGrpc for EclService {
    async fn execute(
        &self,
        request: Request<EclRequest>,
    ) -> Result<Response<EclResponse>, Status> {
        let ecl = &request.into_inner().ecl;

        self.executor.execute(ecl)
            .map(|result| Response::new(EclResponse {
                concept_ids: result.to_vec(),
                count: result.count() as u64,
            }))
            .map_err(|e| Status::invalid_argument(e.to_string()))
    }
}
```

### Batch Processing

```rust
fn process_ecl_batch(
    executor: &EclExecutor<MyStore>,
    queries: &[String],
) -> Vec<Result<Vec<u64>, String>> {
    queries
        .iter()
        .map(|ecl| {
            executor.execute(ecl)
                .map(|r| r.to_vec())
                .map_err(|e| e.to_string())
        })
        .collect()
}
```

### Caching Results

```rust
use std::collections::HashMap;
use std::sync::RwLock;

struct CachedExecutor<'a, T: EclQueryable> {
    executor: EclExecutor<'a, T>,
    cache: RwLock<HashMap<String, Vec<u64>>>,
}

impl<'a, T: EclQueryable> CachedExecutor<'a, T> {
    fn execute(&self, ecl: &str) -> Result<Vec<u64>, EclExecutorError> {
        // Check cache first
        if let Some(result) = self.cache.read().unwrap().get(ecl) {
            return Ok(result.clone());
        }

        // Execute and cache
        let result = self.executor.execute(ecl)?.to_vec();
        self.cache.write().unwrap().insert(ecl.to_string(), result.clone());
        Ok(result)
    }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> TestStore {
        let mut store = TestStore::new();
        store.add_concept(73211009);  // Diabetes
        store.add_concept(46635009);  // Type 1
        store.add_is_a(46635009, 73211009);
        store
    }

    #[test]
    fn test_descendant_query() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("<< 73211009").unwrap();
        assert!(result.to_vec().contains(&73211009));
        assert!(result.to_vec().contains(&46635009));
    }

    #[test]
    fn test_invalid_concept() {
        let store = create_test_store();
        let executor = EclExecutor::new(&store);

        let result = executor.execute("999999999");
        assert!(result.is_err());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_complex_query() {
    let store = load_test_data();
    let executor = EclExecutor::new(&store);

    // Active diabetes types with finding site
    let result = executor.execute(
        "<< 73211009 {{ active = true }} : 363698007 = *"
    ).unwrap();

    assert!(result.count() > 0);
    for id in result.iter() {
        assert!(executor.matches(id, "<< 73211009").unwrap());
    }
}
```

## Performance Tips

1. **Pre-warm cache** with common queries at startup
2. **Use `get_descendants`** instead of ECL for simple cases
3. **Limit results** for exploratory queries
4. **Enable parallelism** for large traversals
5. **Monitor cache hit rates** in production

## Next Steps

- See [TRAIT.md](TRAIT.md) for EclQueryable implementation
- See [../optimizer/README.md](../optimizer/README.md) for performance optimizations
