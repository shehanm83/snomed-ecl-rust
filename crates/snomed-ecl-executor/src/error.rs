//! Error types for ECL execution.

use std::time::Duration;

use snomed_ecl::SctId;
use thiserror::Error;

/// Errors that can occur during ECL execution.
#[derive(Error, Debug)]
pub enum EclExecutorError {
    /// ECL parse error from the snomed-ecl parser.
    #[error("ECL parse error: {0}")]
    ParseError(#[from] snomed_ecl::EclError),

    /// Concept not found in the store.
    #[error("Concept not found: {0}")]
    ConceptNotFound(SctId),

    /// Reference set not found in the store.
    #[error("Reference set not found: {0}")]
    RefsetNotFound(SctId),

    /// Result set exceeds configured limit.
    #[error("Result set too large: {count} exceeds limit {limit}")]
    ResultTooLarge {
        /// Number of results found.
        count: usize,
        /// Configured limit.
        limit: usize,
    },

    /// Query execution timed out.
    #[error("Query timeout after {0:?}")]
    Timeout(Duration),

    /// ECL feature not yet supported by the executor.
    #[error("Unsupported ECL feature: {0}")]
    UnsupportedFeature(String),

    /// Error from the underlying SNOMED store.
    #[error("Store error: {0}")]
    StoreError(String),
}

/// Result type for ECL executor operations.
pub type EclResult<T> = std::result::Result<T, EclExecutorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_concept_not_found() {
        let err = EclExecutorError::ConceptNotFound(73211009);
        assert_eq!(err.to_string(), "Concept not found: 73211009");
    }

    #[test]
    fn test_error_display_refset_not_found() {
        let err = EclExecutorError::RefsetNotFound(700043003);
        assert_eq!(err.to_string(), "Reference set not found: 700043003");
    }

    #[test]
    fn test_error_display_result_too_large() {
        let err = EclExecutorError::ResultTooLarge {
            count: 150000,
            limit: 100000,
        };
        assert_eq!(
            err.to_string(),
            "Result set too large: 150000 exceeds limit 100000"
        );
    }

    #[test]
    fn test_error_display_timeout() {
        let err = EclExecutorError::Timeout(Duration::from_secs(30));
        assert_eq!(err.to_string(), "Query timeout after 30s");
    }

    #[test]
    fn test_error_display_unsupported_feature() {
        let err = EclExecutorError::UnsupportedFeature("refinement filters".to_string());
        assert_eq!(
            err.to_string(),
            "Unsupported ECL feature: refinement filters"
        );
    }

    #[test]
    fn test_error_from_ecl_error() {
        let ecl_err = snomed_ecl::EclError::EmptyExpression;
        let err: EclExecutorError = ecl_err.into();
        assert!(matches!(err, EclExecutorError::ParseError(_)));
    }
}
