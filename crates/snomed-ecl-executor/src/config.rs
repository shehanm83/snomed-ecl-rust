//! Configuration types for the ECL executor.

use std::time::Duration;

/// Configuration for the ECL executor.
///
/// # Example
///
/// ```rust
/// use snomed_ecl_executor::{ExecutorConfig, CacheConfig};
/// use std::time::Duration;
///
/// let config = ExecutorConfig::builder()
///     .with_cache(CacheConfig::default())
///     .with_parallel(true)
///     .with_max_results(100_000)
///     .with_timeout(Duration::from_secs(30))
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ExecutorConfig {
    /// Cache configuration (None = caching disabled).
    pub cache: Option<CacheConfig>,
    /// Enable parallel execution (requires `parallel` feature).
    pub parallel: bool,
    /// Maximum number of results to return (None = unlimited).
    pub max_results: Option<usize>,
    /// Query timeout duration (None = no timeout).
    pub timeout: Option<Duration>,
}

impl ExecutorConfig {
    /// Creates a new builder for ExecutorConfig.
    pub fn builder() -> ExecutorConfigBuilder {
        ExecutorConfigBuilder::default()
    }
}

/// Builder for ExecutorConfig.
#[derive(Debug, Clone, Default)]
pub struct ExecutorConfigBuilder {
    cache: Option<CacheConfig>,
    parallel: bool,
    max_results: Option<usize>,
    timeout: Option<Duration>,
}

impl ExecutorConfigBuilder {
    /// Enables caching with the given configuration.
    pub fn with_cache(mut self, cache: CacheConfig) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Enables or disables parallel execution.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Sets the maximum number of results.
    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = Some(max_results);
        self
    }

    /// Sets the query timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds the ExecutorConfig.
    pub fn build(self) -> ExecutorConfig {
        ExecutorConfig {
            cache: self.cache,
            parallel: self.parallel,
            max_results: self.max_results,
            timeout: self.timeout,
        }
    }
}

/// Configuration for the query cache.
///
/// # Example
///
/// ```rust
/// use snomed_ecl_executor::CacheConfig;
/// use std::time::Duration;
///
/// let cache = CacheConfig {
///     max_entries: 10_000,
///     ttl: Duration::from_secs(300),
///     cache_intermediates: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of cached query results.
    pub max_entries: usize,
    /// Time-to-live for cached entries.
    pub ttl: Duration,
    /// Whether to cache intermediate results during compound query execution.
    pub cache_intermediates: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl: Duration::from_secs(300),
            cache_intermediates: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();
        assert!(config.cache.is_none());
        assert!(!config.parallel);
        assert!(config.max_results.is_none());
        assert!(config.timeout.is_none());
    }

    #[test]
    fn test_executor_config_builder() {
        let config = ExecutorConfig::builder()
            .with_cache(CacheConfig::default())
            .with_parallel(true)
            .with_max_results(50_000)
            .with_timeout(Duration::from_secs(60))
            .build();

        assert!(config.cache.is_some());
        assert!(config.parallel);
        assert_eq!(config.max_results, Some(50_000));
        assert_eq!(config.timeout, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_cache_config_default() {
        let cache = CacheConfig::default();
        assert_eq!(cache.max_entries, 10_000);
        assert_eq!(cache.ttl, Duration::from_secs(300));
        assert!(cache.cache_intermediates);
    }

    #[test]
    fn test_builder_chaining() {
        let config = ExecutorConfig::builder()
            .with_parallel(true)
            .with_max_results(1000)
            .build();

        assert!(config.parallel);
        assert_eq!(config.max_results, Some(1000));
        assert!(config.cache.is_none());
    }
}
