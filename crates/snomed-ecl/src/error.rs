//! Error types for ECL parsing.

use thiserror::Error;

/// Errors that can occur during ECL parsing.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EclError {
    /// Parse error at a specific position in the input.
    #[error("parse error at position {position}: {message}")]
    ParseError {
        /// Position in the input where the error occurred.
        position: usize,
        /// Description of the error.
        message: String,
    },

    /// ECL expression is incomplete.
    #[error("ECL is incomplete: {0}")]
    Incomplete(String),

    /// Unsupported ECL feature.
    #[error("unsupported ECL feature: {feature}")]
    UnsupportedFeature {
        /// Description of the unsupported feature.
        feature: String,
    },

    /// Empty input provided.
    #[error("empty ECL expression")]
    EmptyExpression,

    /// Invalid concept ID format.
    #[error("invalid concept ID: {0}")]
    InvalidConceptId(String),
}

/// Result type for ECL operations.
pub type EclResult<T> = std::result::Result<T, EclError>;
