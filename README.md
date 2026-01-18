# snomed-ecl-rust

A modular Rust implementation of SNOMED CT Expression Constraint Language (ECL).

## Crate Structure

This workspace provides three independent crates that can be used separately or together:

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `snomed-ecl` | **Parser only** - converts ECL strings to AST | `nom`, `thiserror` |
| `snomed-ecl-executor` | **Execution engine** - runs ECL against any data store | `snomed-ecl`, `lru` |
| `snomed-ecl-optimizer` | **Performance optimizations** - transitive closure, bitsets, caching | `snomed-ecl-executor`, optional deps |

```
┌─────────────────────────┐
│      snomed-ecl         │  ← Parser only
│   ECL String → AST      │
└───────────┬─────────────┘
            │ optional
┌───────────▼─────────────┐
│  snomed-ecl-executor    │  ← Execute against EclQueryable trait
│   AST → Query Results   │
└───────────┬─────────────┘
            │ optional
┌───────────▼─────────────┐
│  snomed-ecl-optimizer   │  ← Performance features (feature-gated)
│   O(1) lookups, caching │
└─────────────────────────┘
```

## Usage

### Parser Only (Minimal Dependencies)

Use this when you want to parse ECL and handle execution yourself (e.g., translate to Elasticsearch queries).

```toml
[dependencies]
snomed-ecl = "0.1"
```

```rust
use snomed_ecl::{parse, EclExpression};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parse("<< 404684003 |Clinical finding|")?;

    match ast {
        EclExpression::DescendantOrSelfOf(inner) => {
            println!("Query for descendants of: {:?}", inner);
            // Translate to your backend (Elasticsearch, SQL, etc.)
        }
        EclExpression::And(left, right) => {
            // Handle compound expressions
        }
        // ... handle other variants
        _ => {}
    }

    Ok(())
}
```

### Parser + Executor

Use this when you have a SNOMED CT data store and want built-in execution.

```toml
[dependencies]
snomed-ecl = "0.1"
snomed-ecl-executor = "0.1"
```

```rust
use snomed_ecl_executor::{EclExecutor, EclQueryable};
use snomed_ecl::SctId;

// Implement the trait for your data store
struct MyStore { /* your data */ }

impl EclQueryable for MyStore {
    fn get_children(&self, concept_id: SctId) -> Vec<SctId> {
        // Return direct children from your store
        todo!()
    }

    fn get_parents(&self, concept_id: SctId) -> Vec<SctId> {
        // Return direct parents from your store
        todo!()
    }

    fn has_concept(&self, concept_id: SctId) -> bool {
        // Check if concept exists
        todo!()
    }

    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = SctId> + '_> {
        // Return iterator over all concept IDs
        todo!()
    }

    fn get_refset_members(&self, refset_id: SctId) -> Vec<SctId> {
        // Return members of a reference set
        todo!()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = MyStore { /* ... */ };
    let executor = EclExecutor::new(&store);

    // Execute ECL queries
    let result = executor.execute("<< 404684003")?;
    println!("Found {} concepts", result.count());

    // Check if a concept matches
    let is_clinical_finding = executor.matches(12345678, "<< 404684003")?;

    Ok(())
}
```

### With Performance Optimizations

Use this for production workloads with large SNOMED CT datasets.

```toml
[dependencies]
snomed-ecl = "0.1"
snomed-ecl-executor = "0.1"
snomed-ecl-optimizer = { version = "0.1", features = ["full"] }
```

```rust
use snomed_ecl_optimizer::closure::TransitiveClosure;
use snomed_ecl_executor::EclExecutor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = MyStore { /* ... */ };

    // Build transitive closure for O(1) ancestor/descendant queries
    let closure = TransitiveClosure::build(&store);
    println!("Built closure: {}", closure.stats());

    // Use closure as the store (implements EclQueryable)
    let executor = EclExecutor::new(&closure);

    // Now hierarchy queries are O(1) instead of O(n)
    let result = executor.execute("<< 404684003")?;

    Ok(())
}
```

## Feature Flags (snomed-ecl-optimizer)

| Feature | Description |
|---------|-------------|
| `closure` | Precomputed transitive closure for O(1) hierarchy queries |
| `bitset` | Roaring bitmap-based concept sets |
| `persistence` | Save/load compiled bitsets to disk |
| `filter-service` | Runtime filtering service with LRU caching |
| `full` | Enable all optimizations |

## ECL Support

### Fully Implemented

- Hierarchy operators: `<`, `<<`, `>`, `>>`, `<!`, `<<!`, `>!`, `>>!`
- Compound: `AND`, `OR`, `MINUS`
- Member of: `^`
- Wildcard: `*`
- Nested expressions: `()`
- Attribute refinement: `: attribute = value`
- Attribute groups: `{ }`
- Cardinality: `[min..max]`
- Dot notation: `.`
- Concrete values: `#123`, `#3.14`, `#"string"`
- Top/Bottom of set: `!!>`, `!!<`
- Basic filters: `{{ term = "x" }}`, `{{ active = true }}`

### Partially Implemented

See [docs/ECL_COMPLIANCE_GAPS.md](docs/ECL_COMPLIANCE_GAPS.md) for detailed gap analysis.

## License

Apache-2.0

## References

- [ECL Specification](https://docs.snomed.org/snomed-ct-specifications/snomed-ct-expression-constraint-language)
- [SNOMED CT Expression Constraint Language](https://confluence.ihtsdotools.org/display/DOCECL)
