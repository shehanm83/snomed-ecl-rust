# snomed-ecl Parser

A complete Rust parser for SNOMED CT Expression Constraint Language (ECL) version 2.2.

## What is ECL?

**Expression Constraint Language (ECL)** is a formal query language developed by SNOMED International for selecting sets of clinical concepts from SNOMED CT. Think of it as "SQL for clinical terminology" - it allows you to express complex queries like:

- "Give me all types of diabetes"
- "Find all disorders affecting the heart"
- "Get all active ingredients that treat infections"

### Why ECL Matters

SNOMED CT contains over **350,000 active concepts** organized in a complex hierarchy. Without ECL, querying this data would require:
- Complex recursive SQL queries
- Manual graph traversal code
- Deep knowledge of SNOMED CT's structure

ECL provides a **standardized, portable** way to express these queries that works across different systems and implementations.

## Quick Example

```rust
use snomed_ecl::parse;

// Parse an ECL expression
let ast = parse("<< 73211009 |Diabetes mellitus|").unwrap();

// The AST represents: "Diabetes mellitus and all its subtypes"
// This includes: Type 1 diabetes, Type 2 diabetes, Gestational diabetes, etc.
```

## ECL Specification Overview

ECL is defined by SNOMED International and follows a formal grammar. The current version is **ECL 2.2**.

### Core Concepts

#### 1. SNOMED CT Identifiers (SCTIDs)

Every concept in SNOMED CT has a unique numeric identifier:
- `73211009` - Diabetes mellitus
- `404684003` - Clinical finding
- `363698007` - Finding site (an attribute type)

#### 2. Hierarchy Relationships

SNOMED CT concepts are organized in an "IS-A" hierarchy:
```
Clinical finding (404684003)
  └── Disease (64572001)
        └── Diabetes mellitus (73211009)
              ├── Type 1 diabetes (46635009)
              └── Type 2 diabetes (44054006)
```

#### 3. Attribute Relationships

Concepts have attributes that describe them:
```
Diabetes mellitus
  ├── Finding site = Pancreatic structure
  └── Associated morphology = Abnormality of secretion
```

## ECL Expression Types

### Simple Expressions

| Syntax | Name | Meaning |
|--------|------|---------|
| `73211009` | Self | Exactly this concept |
| `< 73211009` | Descendants | All subtypes (children, grandchildren, etc.) |
| `<< 73211009` | Descendants or Self | The concept plus all subtypes |
| `> 73211009` | Ancestors | All supertypes (parents, grandparents, etc.) |
| `>> 73211009` | Ancestors or Self | The concept plus all supertypes |
| `<! 73211009` | Children | Direct subtypes only |
| `>! 73211009` | Parents | Direct supertypes only |
| `*` | Any | All concepts |
| `^ 700043003` | Member Of | Members of a reference set |

### Compound Expressions

| Syntax | Name | Meaning |
|--------|------|---------|
| `A AND B` | Conjunction | Concepts in both A and B |
| `A OR B` | Disjunction | Concepts in A or B or both |
| `A MINUS B` | Exclusion | Concepts in A but not in B |

### Refinements

Refinements filter concepts based on their attributes:

```
< 404684003 : 363698007 = << 39057004
│             │           │
│             │           └── Value: Pulmonary valve structure (and subtypes)
│             └── Attribute: Finding site
└── Focus: Descendants of Clinical finding
```

### Filters

Filters narrow results based on metadata:

```
<< 73211009 {{ active = true }}
<< 73211009 {{ term = "insulin" }}
<< 73211009 {{ definitionStatus = primitive }}
```

## Parser Architecture

The `snomed-ecl` parser uses the `nom` parser combinator library to build a complete ECL parser:

```
ECL String → Lexer/Parser → Abstract Syntax Tree (AST)
     │                              │
     │                              ▼
"<< 73211009"              EclExpression::DescendantOrSelfOf(
                              Box::new(EclExpression::ConceptReference {
                                  concept_id: 73211009,
                                  term: None
                              })
                           )
```

### AST Structure

The parser produces an `EclExpression` enum with variants for each expression type:

```rust
pub enum EclExpression {
    // Simple expressions
    ConceptReference { concept_id: u64, term: Option<String> },
    DescendantOf(Box<EclExpression>),
    DescendantOrSelfOf(Box<EclExpression>),
    AncestorOf(Box<EclExpression>),
    AncestorOrSelfOf(Box<EclExpression>),
    ChildOf(Box<EclExpression>),
    ParentOf(Box<EclExpression>),
    MemberOf { refset: Box<EclExpression> },
    Any,

    // Compound expressions
    And(Box<EclExpression>, Box<EclExpression>),
    Or(Box<EclExpression>, Box<EclExpression>),
    Minus(Box<EclExpression>, Box<EclExpression>),

    // Advanced features
    Refined { focus: Box<EclExpression>, refinement: Refinement },
    Filtered { expression: Box<EclExpression>, filters: Vec<EclFilter> },
    DotNotation { expression: Box<EclExpression>, attributes: Vec<AttributeOperator> },

    // ... and more
}
```

## Documentation Structure

- **[SYNTAX.md](SYNTAX.md)** - Complete ECL syntax reference with all operators
- **[FILTERS.md](FILTERS.md)** - Detailed guide to all filter types
- **[USAGE.md](USAGE.md)** - How to use the parser in your projects

## Supported ECL Version

This parser supports **ECL 2.2** with the following features:

| Category | Feature | Status |
|----------|---------|--------|
| **Hierarchy** | All operators (<, <<, >, >>, <!, >!, etc.) | ✅ Full |
| **Compound** | AND, OR, MINUS | ✅ Full |
| **Refinement** | Attribute constraints, groups, cardinality | ✅ Full |
| **Filters** | Active, term, definition status, module, etc. | ✅ Full |
| **Concrete Values** | Integer, decimal, string, boolean | ✅ Full |
| **Dot Notation** | Attribute chaining | ✅ Full |
| **History Supplement** | +HISTORY profiles | ✅ Full |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
snomed-ecl = { git = "https://github.com/your-repo/snomed-ecl-rust.git" }
```

## Basic Usage

```rust
use snomed_ecl::{parse, EclExpression};

fn main() -> Result<(), snomed_ecl::EclError> {
    // Parse a simple expression
    let ast = parse("<< 73211009 |Diabetes mellitus|")?;

    // Work with the AST
    match ast {
        EclExpression::DescendantOrSelfOf(inner) => {
            println!("Finding descendants of: {:?}", inner);
        }
        _ => {}
    }

    Ok(())
}
```

## Next Steps

1. Read [SYNTAX.md](SYNTAX.md) for the complete syntax reference
2. Read [FILTERS.md](FILTERS.md) to understand filtering capabilities
3. Read [USAGE.md](USAGE.md) for integration patterns

## References

- [ECL Specification (SNOMED International)](https://confluence.ihtsdotools.org/display/DOCECL)
- [SNOMED CT Documentation](https://confluence.ihtsdotools.org/display/DOCSTART)
