//! Postcoordinated SNOMED CT expression module.
//!
//! This module provides types and utilities for building postcoordinated
//! SNOMED CT expressions using compositional grammar.
//!
//! ## Compositional Grammar vs ECL
//!
//! - **Compositional Grammar** (this module): Builds specific clinical concepts
//!   ```text
//!   29857009 |Chest pain| : 246112005 |Severity| = 24484000 |Severe|
//!   ```
//!
//! - **ECL** (parent module): Defines sets of concepts (constraints)
//!   ```text
//!   << 404684003 |Clinical finding|
//!   ```
//!
//! ## Example
//!
//! ```rust
//! use snomed_ecl::expression::{
//!     Expression, Attribute, ConceptReference, ExpressionOperator,
//!     FluentExpressionBuilder, Format, Formatter,
//! };
//!
//! // Using the fluent builder
//! let expr = FluentExpressionBuilder::new()
//!     .focus_concept(29857009, "Chest pain")
//!     .attribute(246112005, "Severity", 24484000, "Severe")
//!     .build()
//!     .unwrap();
//!
//! // Format in different styles
//! let brief = Formatter::format_expression(&expr, Format::Brief);
//! let long = Formatter::format_expression(&expr, Format::Long);
//!
//! assert_eq!(brief, "29857009:246112005=24484000");
//! assert_eq!(long, "29857009 |Chest pain| : 246112005 |Severity| = 24484000 |Severe|");
//! ```

mod ast;
mod builder;
mod formatter;

// Re-export AST types
pub use ast::{
    Attribute, AttributeValue, ConceptReference, Expression, ExpressionOperator, ExpressionType,
    RoleGroup,
};

// Re-export builder types
pub use builder::{
    AttributeInput, BuildError, BuildRequest, BuildResult, BuildWarning, ConceptInput,
    ExpressionBuilder, FluentExpressionBuilder, RoleGroupBuilder,
};

// Re-export formatter types
pub use formatter::{Format, FormattedExpression, Formatter};
