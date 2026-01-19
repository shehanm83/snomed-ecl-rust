# Parser Usage Guide

How to use the `snomed-ecl` parser in your Rust projects.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
snomed-ecl = { git = "https://github.com/your-repo/snomed-ecl-rust.git" }
```

## Basic Parsing

### Parse an ECL Expression

```rust
use snomed_ecl::{parse, EclExpression};

fn main() {
    // Parse returns Result<EclExpression, EclError>
    match parse("<< 73211009 |Diabetes mellitus|") {
        Ok(ast) => println!("Parsed successfully: {:?}", ast),
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
```

### Working with the AST

```rust
use snomed_ecl::{parse, EclExpression};

fn analyze_expression(ecl: &str) -> Result<(), snomed_ecl::EclError> {
    let ast = parse(ecl)?;

    match ast {
        EclExpression::ConceptReference { concept_id, term } => {
            println!("Single concept: {}", concept_id);
            if let Some(t) = term {
                println!("Term: {}", t);
            }
        }

        EclExpression::DescendantOf(inner) => {
            println!("Descendants of:");
            analyze_inner(&inner);
        }

        EclExpression::DescendantOrSelfOf(inner) => {
            println!("Descendants or self of:");
            analyze_inner(&inner);
        }

        EclExpression::And(left, right) => {
            println!("Conjunction of:");
            println!("  Left: {:?}", left);
            println!("  Right: {:?}", right);
        }

        EclExpression::Or(left, right) => {
            println!("Disjunction of:");
            println!("  Left: {:?}", left);
            println!("  Right: {:?}", right);
        }

        EclExpression::Any => {
            println!("Wildcard - all concepts");
        }

        _ => println!("Other expression type"),
    }

    Ok(())
}

fn analyze_inner(expr: &EclExpression) {
    if let EclExpression::ConceptReference { concept_id, .. } = expr {
        println!("  Concept ID: {}", concept_id);
    }
}
```

## Expression Types

### Hierarchy Operators

```rust
use snomed_ecl::{parse, EclExpression};

fn identify_hierarchy_type(ecl: &str) -> &'static str {
    let ast = parse(ecl).unwrap();

    match ast {
        EclExpression::DescendantOf(_) => "Descendants only",
        EclExpression::DescendantOrSelfOf(_) => "Self + descendants",
        EclExpression::AncestorOf(_) => "Ancestors only",
        EclExpression::AncestorOrSelfOf(_) => "Self + ancestors",
        EclExpression::ChildOf(_) => "Direct children only",
        EclExpression::ParentOf(_) => "Direct parents only",
        EclExpression::ChildOrSelfOf(_) => "Self + children",
        EclExpression::ParentOrSelfOf(_) => "Self + parents",
        _ => "Other",
    }
}

fn main() {
    println!("{}", identify_hierarchy_type("< 73211009"));   // "Descendants only"
    println!("{}", identify_hierarchy_type("<< 73211009"));  // "Self + descendants"
    println!("{}", identify_hierarchy_type("<! 73211009"));  // "Direct children only"
}
```

### Compound Expressions

```rust
use snomed_ecl::{parse, EclExpression};

fn count_operands(ast: &EclExpression) -> usize {
    match ast {
        EclExpression::And(left, right) |
        EclExpression::Or(left, right) |
        EclExpression::Minus(left, right) => {
            count_operands(left) + count_operands(right)
        }
        _ => 1,
    }
}

fn main() {
    let ast = parse("A AND B AND C OR D").unwrap();
    println!("Operand count: {}", count_operands(&ast));
}
```

### Working with Refinements

```rust
use snomed_ecl::{parse, EclExpression, Refinement};

fn extract_attributes(ecl: &str) -> Vec<u64> {
    let ast = parse(ecl).unwrap();

    if let EclExpression::Refined { refinement, .. } = ast {
        refinement
            .ungrouped
            .iter()
            .filter_map(|attr| {
                if let EclExpression::ConceptReference { concept_id, .. } = &*attr.attribute {
                    Some(*concept_id)
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    }
}

fn main() {
    let attrs = extract_attributes("< 404684003 : 363698007 = *, 116676008 = *");
    println!("Attribute types: {:?}", attrs);  // [363698007, 116676008]
}
```

### Working with Filters

```rust
use snomed_ecl::{parse, EclExpression, EclFilter};

fn has_active_filter(ecl: &str) -> bool {
    let ast = parse(ecl).unwrap();

    if let EclExpression::Filtered { filters, .. } = ast {
        filters.iter().any(|f| matches!(f, EclFilter::Active(true)))
    } else {
        false
    }
}

fn extract_term_filters(ecl: &str) -> Vec<String> {
    let ast = parse(ecl).unwrap();

    if let EclExpression::Filtered { filters, .. } = ast {
        filters
            .iter()
            .filter_map(|f| {
                if let EclFilter::Term { terms, .. } = f {
                    Some(terms.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    } else {
        vec![]
    }
}

fn main() {
    println!("{}", has_active_filter("<< 73211009 {{ active = true }}"));  // true

    let terms = extract_term_filters(r#"<< 73211009 {{ term = "insulin" }}"#);
    println!("Terms: {:?}", terms);  // ["insulin"]
}
```

## Error Handling

### Parse Errors

```rust
use snomed_ecl::{parse, EclError};

fn safe_parse(ecl: &str) -> String {
    match parse(ecl) {
        Ok(_) => "Valid ECL".to_string(),
        Err(EclError::ParseError(e)) => {
            format!("Parse error at position {}: {}", e.position, e.message)
        }
    }
}

fn main() {
    println!("{}", safe_parse("<< 73211009"));      // Valid ECL
    println!("{}", safe_parse("<< "));              // Parse error at position 3: ...
    println!("{}", safe_parse("< < 123"));          // Parse error...
}
```

### Validation

```rust
use snomed_ecl::parse;

fn validate_ecl(ecl: &str) -> Result<(), String> {
    parse(ecl).map(|_| ()).map_err(|e| e.to_string())
}

fn validate_batch(expressions: &[&str]) -> Vec<(&str, bool)> {
    expressions
        .iter()
        .map(|ecl| (*ecl, parse(ecl).is_ok()))
        .collect()
}

fn main() {
    let expressions = vec![
        "<< 73211009",
        "< 404684003 : 363698007 = *",
        "invalid << >>",
    ];

    for (ecl, valid) in validate_batch(&expressions) {
        println!("{}: {}", if valid { "✓" } else { "✗" }, ecl);
    }
}
```

## Advanced Usage

### AST Traversal

```rust
use snomed_ecl::{parse, EclExpression};

fn collect_concept_ids(ast: &EclExpression) -> Vec<u64> {
    let mut ids = Vec::new();
    collect_ids_recursive(ast, &mut ids);
    ids
}

fn collect_ids_recursive(ast: &EclExpression, ids: &mut Vec<u64>) {
    match ast {
        EclExpression::ConceptReference { concept_id, .. } => {
            ids.push(*concept_id);
        }
        EclExpression::DescendantOf(inner) |
        EclExpression::DescendantOrSelfOf(inner) |
        EclExpression::AncestorOf(inner) |
        EclExpression::AncestorOrSelfOf(inner) |
        EclExpression::ChildOf(inner) |
        EclExpression::ParentOf(inner) |
        EclExpression::Nested(inner) => {
            collect_ids_recursive(inner, ids);
        }
        EclExpression::And(left, right) |
        EclExpression::Or(left, right) |
        EclExpression::Minus(left, right) => {
            collect_ids_recursive(left, ids);
            collect_ids_recursive(right, ids);
        }
        EclExpression::MemberOf { refset } => {
            collect_ids_recursive(refset, ids);
        }
        EclExpression::Refined { focus, .. } => {
            collect_ids_recursive(focus, ids);
        }
        EclExpression::Filtered { expression, .. } => {
            collect_ids_recursive(expression, ids);
        }
        _ => {}
    }
}

fn main() {
    let ast = parse("<< 73211009 AND << 38341003 OR ^ 700043003").unwrap();
    let ids = collect_concept_ids(&ast);
    println!("Concept IDs: {:?}", ids);  // [73211009, 38341003, 700043003]
}
```

### Expression Complexity Analysis

```rust
use snomed_ecl::{parse, EclExpression};

#[derive(Debug, Default)]
struct Complexity {
    concepts: usize,
    hierarchy_ops: usize,
    compound_ops: usize,
    refinements: usize,
    filters: usize,
}

fn analyze_complexity(ast: &EclExpression) -> Complexity {
    let mut c = Complexity::default();
    analyze_recursive(ast, &mut c);
    c
}

fn analyze_recursive(ast: &EclExpression, c: &mut Complexity) {
    match ast {
        EclExpression::ConceptReference { .. } => c.concepts += 1,
        EclExpression::Any => c.concepts += 1,

        EclExpression::DescendantOf(i) |
        EclExpression::DescendantOrSelfOf(i) |
        EclExpression::AncestorOf(i) |
        EclExpression::AncestorOrSelfOf(i) |
        EclExpression::ChildOf(i) |
        EclExpression::ParentOf(i) => {
            c.hierarchy_ops += 1;
            analyze_recursive(i, c);
        }

        EclExpression::And(l, r) |
        EclExpression::Or(l, r) |
        EclExpression::Minus(l, r) => {
            c.compound_ops += 1;
            analyze_recursive(l, c);
            analyze_recursive(r, c);
        }

        EclExpression::Refined { focus, .. } => {
            c.refinements += 1;
            analyze_recursive(focus, c);
        }

        EclExpression::Filtered { expression, filters, .. } => {
            c.filters += filters.len();
            analyze_recursive(expression, c);
        }

        _ => {}
    }
}

fn main() {
    let ecl = "<< 73211009 AND << 38341003 : 363698007 = * {{ active = true }}";
    let ast = parse(ecl).unwrap();
    let complexity = analyze_complexity(&ast);
    println!("{:?}", complexity);
}
```

### Converting to Other Formats

```rust
use snomed_ecl::{parse, EclExpression};

fn to_elasticsearch_query(ast: &EclExpression) -> serde_json::Value {
    use serde_json::json;

    match ast {
        EclExpression::ConceptReference { concept_id, .. } => {
            json!({
                "term": { "conceptId": concept_id }
            })
        }

        EclExpression::DescendantOrSelfOf(inner) => {
            if let EclExpression::ConceptReference { concept_id, .. } = inner.as_ref() {
                json!({
                    "bool": {
                        "should": [
                            { "term": { "conceptId": concept_id } },
                            { "term": { "ancestors": concept_id } }
                        ]
                    }
                })
            } else {
                json!({})
            }
        }

        EclExpression::And(left, right) => {
            json!({
                "bool": {
                    "must": [
                        to_elasticsearch_query(left),
                        to_elasticsearch_query(right)
                    ]
                }
            })
        }

        EclExpression::Or(left, right) => {
            json!({
                "bool": {
                    "should": [
                        to_elasticsearch_query(left),
                        to_elasticsearch_query(right)
                    ]
                }
            })
        }

        _ => json!({}),
    }
}
```

## Display and Formatting

### Convert AST Back to ECL String

```rust
use snomed_ecl::{parse, EclExpression};

fn main() {
    let ast = parse("<< 73211009 |Diabetes mellitus|").unwrap();

    // EclExpression implements Display
    let ecl_string = ast.to_string();
    println!("{}", ecl_string);  // << 73211009 |Diabetes mellitus|
}
```

### Pretty Printing

```rust
use snomed_ecl::{parse, EclExpression};

fn pretty_print(ast: &EclExpression, indent: usize) {
    let prefix = "  ".repeat(indent);

    match ast {
        EclExpression::ConceptReference { concept_id, term } => {
            print!("{}{}", prefix, concept_id);
            if let Some(t) = term {
                print!(" |{}|", t);
            }
            println!();
        }

        EclExpression::DescendantOrSelfOf(inner) => {
            println!("{}<<", prefix);
            pretty_print(inner, indent + 1);
        }

        EclExpression::And(left, right) => {
            println!("{}AND", prefix);
            pretty_print(left, indent + 1);
            pretty_print(right, indent + 1);
        }

        _ => println!("{}{:?}", prefix, ast),
    }
}

fn main() {
    let ast = parse("<< 73211009 AND << 38341003").unwrap();
    pretty_print(&ast, 0);
}
```

## Integration Patterns

### With Executor

```rust
use snomed_ecl::parse;
use snomed_ecl_executor::EclExecutor;

fn execute_validated_ecl<T: snomed_ecl_executor::EclQueryable>(
    store: &T,
    ecl: &str,
) -> Result<Vec<u64>, String> {
    // First parse to validate
    let _ast = parse(ecl).map_err(|e| format!("Invalid ECL: {}", e))?;

    // Then execute
    let executor = EclExecutor::new(store);
    executor
        .execute(ecl)
        .map(|r| r.to_vec())
        .map_err(|e| format!("Execution error: {}", e))
}
```

### Caching Parsed Expressions

```rust
use std::collections::HashMap;
use snomed_ecl::{parse, EclExpression};

struct EclCache {
    cache: HashMap<String, EclExpression>,
}

impl EclCache {
    fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    fn get_or_parse(&mut self, ecl: &str) -> Result<&EclExpression, snomed_ecl::EclError> {
        if !self.cache.contains_key(ecl) {
            let ast = parse(ecl)?;
            self.cache.insert(ecl.to_string(), ast);
        }
        Ok(self.cache.get(ecl).unwrap())
    }
}
```

## Performance Considerations

1. **Parse once, use many times** - Parsing is relatively fast but caching ASTs is faster
2. **Validate early** - Parse user input at API boundaries
3. **Use Display sparingly** - Regenerating ECL strings has overhead

## Next Steps

- See [SYNTAX.md](SYNTAX.md) for complete syntax reference
- See [FILTERS.md](FILTERS.md) for filter documentation
- See [../executor/README.md](../executor/README.md) for query execution
