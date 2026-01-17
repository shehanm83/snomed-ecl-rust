//! # snomed-ecl-executor
//!
//! High-performance ECL execution engine for SNOMED CT.
//!
//! This crate provides a **standalone** ECL executor that uses the [`snomed-ecl`]
//! parser to execute Expression Constraint Language (ECL) queries against any
//! SNOMED CT store that implements the [`EclQueryable`] trait.
//!
//! ## Key Features
//!
//! - **Store-agnostic** - Works with any store implementing [`EclQueryable`]
//! - **Sub-second queries** - Execute ECL over 3M+ concepts in <100ms
//! - **Configurable caching** - LRU cache for frequently-used queries
//! - **Optional parallelism** - Enable `parallel` feature for multi-threaded traversal
//!
//! ## Quick Start
//!
//! ```ignore
//! use snomed_ecl_executor::{EclExecutor, EclQueryable, SctId};
//!
//! // Your store must implement EclQueryable (see trait documentation)
//! let store = MyStore::new();
//!
//! // Create executor
//! let executor = EclExecutor::new(&store);
//!
//! // Execute ECL query
//! let result = executor.execute("<< 73211009 |Diabetes mellitus|")?;
//! println!("Found {} diabetes-related concepts", result.count());
//!
//! // Check if a concept matches a constraint
//! if result.contains(46635009) {  // Type 2 diabetes
//!     println!("Type 2 diabetes is included");
//! }
//! ```
//!
//! ## With Configuration
//!
//! ```ignore
//! use snomed_ecl_executor::{EclExecutor, ExecutorConfig, CacheConfig};
//! use std::time::Duration;
//!
//! let config = ExecutorConfig::builder()
//!     .with_cache(CacheConfig {
//!         max_entries: 10_000,
//!         ttl: Duration::from_secs(300),
//!         cache_intermediates: true,
//!     })
//!     .with_parallel(true)
//!     .with_max_results(100_000)
//!     .build();
//!
//! let executor = EclExecutor::with_config(&store, config);
//! ```
//!
//! ## Supported ECL Features
//!
//! | Operator | Example | Supported |
//! |----------|---------|-----------|
//! | Self | `73211009` | Yes |
//! | Descendant of | `< 73211009` | Yes |
//! | Descendant or self | `<< 73211009` | Yes |
//! | Ancestor of | `> 73211009` | Yes |
//! | Ancestor or self | `>> 73211009` | Yes |
//! | Child of | `<! 73211009` | Yes |
//! | Parent of | `>! 73211009` | Yes |
//! | AND | `A AND B` | Yes |
//! | OR | `A OR B` | Yes |
//! | MINUS | `A MINUS B` | Yes |
//! | Member of | `^ 700043003` | Partial |
//! | Any | `*` | Yes |
//!
//! ## Feature Flags
//!
//! - `parallel` - Enables parallel query execution using rayon
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    snomed-ecl-executor                       │
//! │                                                              │
//! │  EclExecutor<T: EclQueryable>                                │
//! │  ├── parse ECL string → EclExpression (snomed-ecl)          │
//! │  ├── traverse hierarchy (via EclQueryable trait)            │
//! │  ├── apply set operations (AND/OR/MINUS)                    │
//! │  └── return QueryResult with stats                          │
//! │                                                              │
//! │  Your Application:                                           │
//! │  └── impl EclQueryable for YourStore { ... }                │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod cache;
mod config;
mod error;
mod executor;
mod planner;
mod result;
mod statistics;
mod traits;
mod traverser;

// Public re-exports
pub use cache::{normalize_cache_key, CacheStats, QueryCache};
pub use config::{CacheConfig, ExecutorConfig, ExecutorConfigBuilder};
pub use error::{EclExecutorError, EclResult};
pub use executor::EclExecutor;
pub use planner::{QueryPlan, QueryPlanner, QueryStep};
pub use result::{ExecutionStats, QueryResult};
pub use statistics::{cost, heuristics, well_known, StatisticsService};
pub use traits::{
    ConcreteRelationshipInfo, ConcreteValueRef, DescriptionInfo, EclQueryable, RelationshipInfo,
};
pub use traverser::HierarchyTraverser;

// Re-export commonly used types from snomed-ecl for convenience
pub use snomed_ecl::{EclExpression, SctId};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api_accessible() {
        // Verify all public types are accessible
        let _: Option<CacheConfig> = None;
        let _: Option<ExecutorConfig> = None;
        let _: Option<QueryResult> = None;
        let _: Option<ExecutionStats> = None;
        let _: Option<EclResult<()>> = None;
    }

    #[test]
    fn test_re_exports() {
        // Verify re-exports work
        let _id: SctId = 73211009;
        let _ = snomed_ecl::parse("<< 73211009");
    }
}
