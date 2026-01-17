//! Error types for the optimizer crate.

use snomed_ecl::SctId;

#[cfg(feature = "persistence")]
use std::path::PathBuf;

/// Result type for optimizer operations.
pub type OptimizerResult<T> = Result<T, OptimizerError>;

/// Errors that can occur during optimizer operations.
#[derive(Debug, thiserror::Error)]
pub enum OptimizerError {
    /// ECL parsing error.
    #[error("ECL parse error: {0}")]
    ParseError(#[from] snomed_ecl::EclError),

    /// ECL execution error.
    #[error("ECL execution error: {0}")]
    ExecutionError(#[from] snomed_ecl_executor::EclExecutorError),

    /// Concept not found in registry.
    #[error("Concept {0} not found in registry")]
    ConceptNotFound(SctId),

    /// Registry mismatch between bitsets.
    #[error("Registry mismatch: bitsets use different registries")]
    RegistryMismatch,

    /// I/O error during persistence operations.
    #[cfg(feature = "persistence")]
    #[error("I/O error at {path}: {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Invalid file format during load.
    #[cfg(feature = "persistence")]
    #[error("Invalid file format: {message}")]
    InvalidFormat { message: String },

    /// Serialization error.
    #[cfg(feature = "persistence")]
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error.
    #[cfg(feature = "persistence")]
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Hash mismatch during validation.
    #[cfg(feature = "persistence")]
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// Filter service error.
    #[cfg(feature = "filter-service")]
    #[error("Filter error: {0}")]
    FilterError(String),

    /// No constraint loaded for attribute.
    #[cfg(feature = "filter-service")]
    #[error("No constraint loaded for attribute {attribute_id}")]
    NoConstraint { attribute_id: String },

    /// Build error during closure construction.
    #[cfg(feature = "closure")]
    #[error("Closure build error: {0}")]
    ClosureBuildError(String),
}

impl OptimizerError {
    /// Creates an I/O error with path context.
    #[cfg(feature = "persistence")]
    pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::IoError {
            path: path.into(),
            source,
        }
    }

    /// Creates an invalid format error.
    #[cfg(feature = "persistence")]
    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormat {
            message: message.into(),
        }
    }
}
