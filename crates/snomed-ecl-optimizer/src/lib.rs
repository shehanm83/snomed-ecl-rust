//! # snomed-ecl-optimizer
//!
//! Performance optimizations for the `snomed-ecl-executor` crate.
//!
//! This crate provides optional, feature-gated performance optimizations
//! that can significantly improve ECL query execution performance.
//!
//! ## Features
//!
//! Each optimization is independently feature-gated:
//!
//! - **`closure`**: Precomputed transitive closure for O(1) ancestor/descendant lookups
//! - **`bitset`**: Roaring bitmap-based concept sets for memory-efficient storage
//! - **`persistence`**: Save/load compiled bitsets to disk
//! - **`filter-service`**: Runtime ECL filtering service with caching
//! - **`full`**: Enable all optimizations
//!
//! ## Quick Start
//!
//! ### Using TransitiveClosure (feature: `closure`)
//!
//! ```ignore
//! use snomed_ecl_optimizer::closure::TransitiveClosure;
//! use snomed_ecl_executor::EclExecutor;
//!
//! // Build closure from your store (one-time operation)
//! let closure = TransitiveClosure::build(&my_store);
//!
//! // Use closure directly with executor for O(1) hierarchy queries
//! let executor = EclExecutor::new(&closure);
//! let result = executor.execute("<< 73211009")?;
//! ```
//!
//! ### Using ConceptBitSet (feature: `bitset`)
//!
//! ```ignore
//! use snomed_ecl_optimizer::bitset::{ConceptBitSet, ConceptIdRegistry};
//!
//! // Create registry from concepts
//! let registry = ConceptIdRegistry::from_concepts(store.all_concept_ids());
//!
//! // Create bitsets for efficient set operations
//! let set1 = ConceptBitSet::from_hash_set(&descendants1, registry.clone());
//! let set2 = ConceptBitSet::from_hash_set(&descendants2, registry.clone());
//!
//! // Fast intersection (AND operation)
//! let intersection = set1.intersection(&set2);
//! ```
//!
//! ### Using EclFilterService (feature: `filter-service`)
//!
//! ```ignore
//! use snomed_ecl_optimizer::service::EclFilterService;
//!
//! let service = EclFilterService::new(&store);
//!
//! // Filter candidates by ECL constraint
//! let valid = service.filter(&candidates, "<< 404684003")?;
//!
//! // Check single concept
//! if service.matches(concept_id, "<< 73211009")? {
//!     // Concept is a type of diabetes
//! }
//! ```

pub mod error;

// Feature-gated modules
#[cfg(feature = "closure")]
pub mod closure;

#[cfg(feature = "bitset")]
pub mod bitset;

#[cfg(feature = "persistence")]
pub mod persistence;

#[cfg(feature = "filter-service")]
pub mod service;

// Re-export commonly used types
pub use error::{OptimizerError, OptimizerResult};

#[cfg(feature = "closure")]
pub use closure::TransitiveClosure;

#[cfg(feature = "bitset")]
pub use bitset::{ConceptBitSet, ConceptIdRegistry};

#[cfg(feature = "filter-service")]
pub use service::EclFilterService;

// Re-export from snomed-ecl for convenience
pub use snomed_ecl::SctId;
