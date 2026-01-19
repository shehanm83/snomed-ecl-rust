# snomed-ecl-rust

A modular Rust implementation of SNOMED CT Expression Constraint Language (ECL) version 2.2.

## What is ECL?

**Expression Constraint Language (ECL)** is a formal query language for selecting sets of clinical concepts from SNOMED CT. Think of it as "SQL for medical terminology."

```rust
use snomed_ecl::parse;

// Parse ECL expression
let ast = parse("<< 73211009 |Diabetes mellitus|")?;

// This represents: "Diabetes and all its subtypes"
// Including: Type 1 diabetes, Type 2 diabetes, Gestational diabetes, etc.
```

## Crate Structure

This workspace provides three independent crates that can be used separately or together:

| Crate | Purpose | Use Case |
|-------|---------|----------|
| [`snomed-ecl`](docs/parser/README.md) | **Parser** - ECL strings to AST | Translate ECL to SQL/Elasticsearch |
| [`snomed-ecl-executor`](docs/executor/README.md) | **Executor** - Run ECL queries | Query any SNOMED CT store |
| [`snomed-ecl-optimizer`](docs/optimizer/README.md) | **Optimizer** - Performance | Production with 350k+ concepts |

```
┌─────────────────────────┐
│      snomed-ecl         │  ← Parser only (no dependencies on data)
│   ECL String → AST      │
└───────────┬─────────────┘
            │ optional
┌───────────▼─────────────┐
│  snomed-ecl-executor    │  ← Execute against any EclQueryable store
│   AST → Query Results   │
└───────────┬─────────────┘
            │ optional
┌───────────▼─────────────┐
│  snomed-ecl-optimizer   │  ← O(1) queries, bitmap operations
│   Performance features  │
└─────────────────────────┘
```

## Quick Start

### Parser Only

```toml
[dependencies]
snomed-ecl = { git = "https://github.com/your-repo/snomed-ecl-rust.git" }
```

```rust
use snomed_ecl::{parse, EclExpression};

let ast = parse("<< 404684003 |Clinical finding|")?;

match ast {
    EclExpression::DescendantOrSelfOf(inner) => {
        // Translate to your backend
    }
    _ => {}
}
```

### Parser + Executor

```toml
[dependencies]
snomed-ecl-executor = { git = "https://github.com/your-repo/snomed-ecl-rust.git" }
```

```rust
use snomed_ecl_executor::{EclExecutor, EclQueryable};

// Implement EclQueryable for your data store
impl EclQueryable for MyStore {
    fn get_children(&self, id: u64) -> Vec<u64> { /* ... */ }
    fn get_parents(&self, id: u64) -> Vec<u64> { /* ... */ }
    fn has_concept(&self, id: u64) -> bool { /* ... */ }
    fn all_concept_ids(&self) -> Box<dyn Iterator<Item = u64> + '_> { /* ... */ }
    fn get_refset_members(&self, id: u64) -> Vec<u64> { vec![] }
}

let executor = EclExecutor::new(&store);
let result = executor.execute("<< 73211009")?;
println!("Found {} diabetes concepts", result.count());
```

### With Performance Optimizations

```toml
[dependencies]
snomed-ecl-optimizer = { git = "...", features = ["full"] }
```

```rust
use snomed_ecl_optimizer::closure::TransitiveClosure;

// Build transitive closure (one-time)
let closure = TransitiveClosure::build(&store);

// Now hierarchy queries are O(1) instead of O(n)
let executor = EclExecutor::new(&closure);
let result = executor.execute("<< 404684003")?;  // Instant!
```

## ECL Support (v2.2)

| Category | Features | Status |
|----------|----------|--------|
| **Hierarchy** | `<` `<<` `>` `>>` `<!` `>!` `<<!` `>>!` | ✅ Full |
| **Compound** | `AND` `OR` `MINUS` `()` | ✅ Full |
| **Member Of** | `^` with nested expressions | ✅ Full |
| **Refinement** | `: attr = value`, groups `{ }`, cardinality `[n..m]` | ✅ Full |
| **Filters** | `active`, `term`, `moduleId`, `definitionStatus`, etc. | ✅ Full |
| **Concrete Values** | `#123` `#3.14` `#"string"` `#true` | ✅ Full |
| **Dot Notation** | `. attribute` chaining | ✅ Full |
| **History** | `+HISTORY-MIN/MOD/MAX` | ✅ Full |
| **Alternate IDs** | `http://snomed.info/id/123` | ✅ Full |

## Documentation

### Parser (`snomed-ecl`)
- [Overview & ECL Introduction](docs/parser/README.md) - What is ECL, specification overview
- [Syntax Reference](docs/parser/SYNTAX.md) - Complete ECL syntax with examples
- [Filters Guide](docs/parser/FILTERS.md) - All filter types explained
- [Usage Guide](docs/parser/USAGE.md) - Code examples and patterns

### Executor (`snomed-ecl-executor`)
- [Overview & Architecture](docs/executor/README.md) - How the executor works
- [EclQueryable Trait](docs/executor/TRAIT.md) - Implementing the trait
- [Usage Guide](docs/executor/USAGE.md) - Query examples, configuration, integration

### Optimizer (`snomed-ecl-optimizer`)
- [Performance Guide](docs/optimizer/README.md) - Closure, bitmaps, persistence

## Feature Flags

| Feature | Crate | Description |
|---------|-------|-------------|
| `closure` | optimizer | Precomputed transitive closure |
| `bitset` | optimizer | Roaring bitmap operations |
| `persistence` | optimizer | Save/load to disk |
| `filter-service` | optimizer | Filter result caching |
| `full` | optimizer | All optimizations |

## Test Coverage

```
353 tests passing
├── 161 parser tests
├── 135 executor unit tests
├── 34 integration tests
├── 10 filter tests
├── 6 syntax tests
└── 7 doc tests
```

## License

Apache-2.0

## References

- [ECL Specification](https://docs.snomed.org/snomed-ct-specifications/snomed-ct-expression-constraint-language)
- [SNOMED CT Documentation](https://confluence.ihtsdotools.org/display/DOCECL)
