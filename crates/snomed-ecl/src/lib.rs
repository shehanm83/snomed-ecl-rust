//! # snomed-ecl
//!
//! A Rust library for SNOMED CT Expression Constraint Language (ECL) and
//! postcoordinated expression building.
//!
//! This crate provides:
//! - **ECL Parser**: Parse ECL constraint expressions for querying concept sets
//! - **Expression Builder**: Build postcoordinated SNOMED CT expressions
//!
//! ## ECL vs Compositional Grammar
//!
//! | Feature | ECL (Constraints) | Compositional Grammar |
//! |---------|-------------------|----------------------|
//! | Purpose | Query/filter concept sets | Build specific concepts |
//! | Example | `<< 404684003` | `29857009 : 246112005 = 24484000` |
//! | Use case | "Find all clinical findings" | "Severe chest pain" |
//!
//! ## ECL Usage
//!
//! ```rust
//! use snomed_ecl::{parse, EclExpression};
//!
//! // Parse a simple descendant constraint
//! let expr = parse("<< 404684003 |Clinical finding|").unwrap();
//!
//! // Parse a compound expression
//! let expr = parse("< 19829001 AND < 301867009").unwrap();
//! ```
//!
//! ## Expression Builder Usage
//!
//! ```rust
//! use snomed_ecl::expression::{FluentExpressionBuilder, Format, Formatter};
//!
//! // Build a postcoordinated expression
//! let expr = FluentExpressionBuilder::new()
//!     .focus_concept(29857009, "Chest pain")
//!     .attribute(246112005, "Severity", 24484000, "Severe")
//!     .build()
//!     .unwrap();
//!
//! // Format it
//! let ecl = Formatter::format_expression(&expr, Format::Long);
//! assert_eq!(ecl, "29857009 |Chest pain| : 246112005 |Severity| = 24484000 |Severe|");
//! ```
//!
//! ## ECL Syntax Quick Reference
//!
//! | Operator | Meaning | Example |
//! |----------|---------|---------|
//! | (none) | Self | `404684003` |
//! | `<` | Descendants of | `< 404684003` |
//! | `<<` | Descendants or self of | `<< 404684003` |
//! | `>` | Ancestors of | `> 404684003` |
//! | `>>` | Ancestors or self of | `>> 404684003` |
//! | `^` | Member of (refset) | `^ 700043003` |
//! | `*` | Any concept | `*` |
//! | `AND` | Conjunction | `<< A AND << B` |
//! | `OR` | Disjunction | `<< A OR << B` |
//! | `MINUS` | Exclusion | `<< A MINUS << B` |

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod ast;
mod error;
pub mod expression;
mod parser;

pub use ast::{
    AttributeConstraint, AttributeGroup, Cardinality, ComparisonOperator, ConcreteValue,
    EclExpression, EclFilter, FilterAcceptability, HistoryProfile, MemberFieldValue,
    Refinement, RefinementOperator, TermMatchType,
};
pub use error::{EclError, EclResult};
pub use parser::parse;

/// SNOMED CT Identifier type (64-bit unsigned integer).
pub type SctId = u64;
