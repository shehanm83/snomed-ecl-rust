//! Types for the filter service.

use snomed_ecl::SctId;

/// Result of a filter operation.
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// Concept IDs that passed the filter.
    pub filtered_ids: Vec<SctId>,
    /// Number of candidates before filtering.
    pub original_count: usize,
    /// Time taken to filter in milliseconds.
    pub filter_time_ms: f64,
}

impl FilterResult {
    /// Returns the number of concepts that passed the filter.
    pub fn filtered_count(&self) -> usize {
        self.filtered_ids.len()
    }

    /// Returns true if no concepts passed the filter.
    pub fn is_empty(&self) -> bool {
        self.filtered_ids.is_empty()
    }

    /// Returns the filter pass rate as a percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.original_count == 0 {
            0.0
        } else {
            (self.filtered_ids.len() as f64 / self.original_count as f64) * 100.0
        }
    }
}

/// Statistics about filter service usage.
#[derive(Debug, Clone, Default)]
pub struct FilterStats {
    /// Number of queries executed.
    pub queries_executed: usize,
    /// Number of cache hits.
    pub cache_hits: usize,
    /// Number of cache misses.
    pub cache_misses: usize,
    /// Total time spent filtering in milliseconds.
    pub total_filter_time_ms: f64,
    /// Total number of candidates filtered.
    pub total_candidates_filtered: usize,
}

impl FilterStats {
    /// Returns the cache hit rate as a percentage.
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / total as f64) * 100.0
        }
    }

    /// Returns the average filter time in milliseconds.
    pub fn avg_filter_time_ms(&self) -> f64 {
        if self.queries_executed == 0 {
            0.0
        } else {
            self.total_filter_time_ms / self.queries_executed as f64
        }
    }
}

impl std::fmt::Display for FilterStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Filter Service Statistics:")?;
        writeln!(f, "  Queries:         {}", self.queries_executed)?;
        writeln!(f, "  Cache hits:      {}", self.cache_hits)?;
        writeln!(f, "  Cache misses:    {}", self.cache_misses)?;
        writeln!(f, "  Hit rate:        {:.1}%", self.cache_hit_rate())?;
        writeln!(f, "  Total time:      {:.1}ms", self.total_filter_time_ms)?;
        writeln!(f, "  Avg time:        {:.2}ms", self.avg_filter_time_ms())?;
        writeln!(f, "  Candidates:      {}", self.total_candidates_filtered)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_result() {
        let result = FilterResult {
            filtered_ids: vec![100, 200, 300],
            original_count: 10,
            filter_time_ms: 1.5,
        };

        assert_eq!(result.filtered_count(), 3);
        assert!(!result.is_empty());
        assert!((result.pass_rate() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_filter_result_empty() {
        let result = FilterResult {
            filtered_ids: vec![],
            original_count: 10,
            filter_time_ms: 1.0,
        };

        assert!(result.is_empty());
        assert!((result.pass_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_filter_stats() {
        let stats = FilterStats {
            queries_executed: 100,
            cache_hits: 75,
            cache_misses: 25,
            total_filter_time_ms: 500.0,
            total_candidates_filtered: 10000,
        };

        assert!((stats.cache_hit_rate() - 75.0).abs() < 0.01);
        assert!((stats.avg_filter_time_ms() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_filter_stats_empty() {
        let stats = FilterStats::default();

        assert!((stats.cache_hit_rate() - 0.0).abs() < 0.01);
        assert!((stats.avg_filter_time_ms() - 0.0).abs() < 0.01);
    }
}
