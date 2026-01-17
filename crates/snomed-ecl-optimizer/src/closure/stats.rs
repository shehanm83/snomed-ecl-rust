//! Statistics about transitive closure builds.

/// Statistics about the transitive closure.
#[derive(Debug, Clone)]
pub struct ClosureStats {
    /// Number of concepts in the closure.
    pub concept_count: usize,
    /// Number of IS-A relationships processed.
    pub relationship_count: usize,
    /// Maximum depth of the hierarchy.
    pub max_hierarchy_depth: usize,
    /// Average number of ancestors per concept.
    pub avg_ancestors: f64,
    /// Average number of descendants per concept.
    pub avg_descendants: f64,
    /// Time taken to build the closure in milliseconds.
    pub build_time_ms: u64,
    /// Estimated memory usage in bytes.
    pub memory_estimate_bytes: usize,
}

impl ClosureStats {
    /// Returns estimated memory usage in megabytes.
    pub fn memory_mb(&self) -> f64 {
        self.memory_estimate_bytes as f64 / (1024.0 * 1024.0)
    }
}

impl std::fmt::Display for ClosureStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Transitive Closure Statistics:")?;
        writeln!(f, "  Concepts:        {}", self.concept_count)?;
        writeln!(f, "  IS-A relations:  {}", self.relationship_count)?;
        writeln!(f, "  Max depth:       {}", self.max_hierarchy_depth)?;
        writeln!(f, "  Avg ancestors:   {:.1}", self.avg_ancestors)?;
        writeln!(f, "  Avg descendants: {:.1}", self.avg_descendants)?;
        writeln!(f, "  Build time:      {}ms", self.build_time_ms)?;
        writeln!(f, "  Memory estimate: {:.1} MB", self.memory_mb())?;
        Ok(())
    }
}
