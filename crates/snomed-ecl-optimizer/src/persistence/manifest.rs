//! Manifest file for a collection of compiled bitsets.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use crate::error::{OptimizerError, OptimizerResult};

/// Manifest file for a collection of compiled bitsets.
///
/// The manifest tracks metadata about all bitsets in a directory,
/// including the SNOMED release version, compilation timestamp,
/// and per-bitset statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitsetManifest {
    /// SNOMED CT release version.
    pub snomed_release: String,
    /// Timestamp when bitsets were compiled.
    pub compiled_at: DateTime<Utc>,
    /// Version of the compilation tooling.
    pub compiler_version: String,
    /// Total number of concepts in the registry.
    pub total_concepts: usize,
    /// Per-bitset entries.
    pub bitsets: Vec<BitsetEntry>,
}

/// Entry for a single bitset in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitsetEntry {
    /// Identifier for this bitset (e.g., attribute ID).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Type of constraint.
    pub constraint_type: ConstraintType,
    /// Original ECL expression.
    pub ecl_expression: String,
    /// Number of concepts matching the constraint.
    pub concept_count: u64,
    /// File size in bytes.
    pub file_size_bytes: u64,
    /// Filename (relative to manifest).
    pub filename: String,
}

/// Type of constraint for a bitset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConstraintType {
    /// Domain constraint (concepts where attribute applies).
    Domain,
    /// Range constraint (valid values for attribute).
    Range,
    /// General ECL constraint.
    General,
}

impl BitsetManifest {
    /// Creates a new empty manifest.
    pub fn new(snomed_release: &str, total_concepts: usize) -> Self {
        Self {
            snomed_release: snomed_release.to_string(),
            compiled_at: Utc::now(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            total_concepts,
            bitsets: Vec::new(),
        }
    }

    /// Adds a bitset entry to the manifest.
    pub fn add_entry(&mut self, entry: BitsetEntry) {
        self.bitsets.push(entry);
    }

    /// Returns the number of bitsets in the manifest.
    pub fn count(&self) -> usize {
        self.bitsets.len()
    }

    /// Returns the total file size of all bitsets.
    pub fn total_size_bytes(&self) -> u64 {
        self.bitsets.iter().map(|b| b.file_size_bytes).sum()
    }

    /// Returns the total number of concepts across all bitsets.
    pub fn total_concept_entries(&self) -> u64 {
        self.bitsets.iter().map(|b| b.concept_count).sum()
    }

    /// Finds a bitset entry by ID.
    pub fn get_entry(&self, id: &str) -> Option<&BitsetEntry> {
        self.bitsets.iter().find(|e| e.id == id)
    }

    /// Saves the manifest to a JSON file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> OptimizerResult<()> {
        let path = path.as_ref();
        let file = File::create(path).map_err(|e| OptimizerError::io_error(path, e))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)
            .map_err(|e| OptimizerError::SerializationError(e.to_string()))?;
        Ok(())
    }

    /// Loads a manifest from a JSON file.
    pub fn load<P: AsRef<Path>>(path: P) -> OptimizerResult<Self> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| OptimizerError::io_error(path, e))?;
        let reader = BufReader::new(file);
        let manifest: Self = serde_json::from_reader(reader)
            .map_err(|e| OptimizerError::DeserializationError(e.to_string()))?;
        Ok(manifest)
    }
}

impl std::fmt::Display for BitsetManifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Bitset Manifest")?;
        writeln!(f, "  SNOMED Release:  {}", self.snomed_release)?;
        writeln!(f, "  Compiled:        {}", self.compiled_at)?;
        writeln!(f, "  Compiler:        {}", self.compiler_version)?;
        writeln!(f, "  Total Concepts:  {}", self.total_concepts)?;
        writeln!(f, "  Bitset Count:    {}", self.count())?;
        writeln!(
            f,
            "  Total Size:      {} KB",
            self.total_size_bytes() / 1024
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_creation() {
        let manifest = BitsetManifest::new("20240101", 500000);

        assert_eq!(manifest.snomed_release, "20240101");
        assert_eq!(manifest.total_concepts, 500000);
        assert_eq!(manifest.count(), 0);
    }

    #[test]
    fn test_add_entry() {
        let mut manifest = BitsetManifest::new("20240101", 500000);

        manifest.add_entry(BitsetEntry {
            id: "363698007".to_string(),
            name: "Finding site".to_string(),
            constraint_type: ConstraintType::Range,
            ecl_expression: "<< 123037004".to_string(),
            concept_count: 50000,
            file_size_bytes: 10240,
            filename: "363698007.eclb".to_string(),
        });

        assert_eq!(manifest.count(), 1);
        assert_eq!(manifest.total_size_bytes(), 10240);
        assert_eq!(manifest.total_concept_entries(), 50000);
    }

    #[test]
    fn test_get_entry() {
        let mut manifest = BitsetManifest::new("20240101", 500000);

        manifest.add_entry(BitsetEntry {
            id: "363698007".to_string(),
            name: "Finding site".to_string(),
            constraint_type: ConstraintType::Range,
            ecl_expression: "<< 123037004".to_string(),
            concept_count: 50000,
            file_size_bytes: 10240,
            filename: "363698007.eclb".to_string(),
        });

        let entry = manifest.get_entry("363698007").unwrap();
        assert_eq!(entry.name, "Finding site");

        assert!(manifest.get_entry("unknown").is_none());
    }

    #[test]
    fn test_save_and_load() {
        let mut manifest = BitsetManifest::new("20240101", 500000);

        manifest.add_entry(BitsetEntry {
            id: "363698007".to_string(),
            name: "Finding site".to_string(),
            constraint_type: ConstraintType::Range,
            ecl_expression: "<< 123037004".to_string(),
            concept_count: 50000,
            file_size_bytes: 10240,
            filename: "363698007.eclb".to_string(),
        });

        let dir = tempdir().unwrap();
        let path = dir.path().join("manifest.json");

        manifest.save(&path).unwrap();
        let loaded = BitsetManifest::load(&path).unwrap();

        assert_eq!(loaded.snomed_release, "20240101");
        assert_eq!(loaded.count(), 1);
        assert_eq!(loaded.bitsets[0].id, "363698007");
    }
}
