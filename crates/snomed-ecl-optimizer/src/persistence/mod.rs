//! Save/load compiled bitsets to/from disk.
//!
//! This module provides persistence for precompiled ECL bitsets,
//! allowing them to be saved after initial compilation and loaded
//! quickly on subsequent runs.
//!
//! # File Format
//!
//! The bitset file format (`.eclb`) is a binary format:
//!
//! ```text
//! [4 bytes]  Magic: "ECLB"
//! [4 bytes]  Version (u32 LE)
//! [4 bytes]  ECL expression length (u32 LE)
//! [var]      ECL expression (UTF-8)
//! [8 bytes]  Concept count (u64 LE)
//! [32 bytes] SHA-256 hash of ECL expression
//! [4 bytes]  Bitmap data length (u32 LE)
//! [var]      Serialized roaring bitmap
//! ```
//!
//! # Example
//!
//! ```ignore
//! use snomed_ecl_optimizer::persistence::{BitsetFile, BitsetManifest};
//! use snomed_ecl_optimizer::bitset::{ConceptBitSet, ConceptIdRegistry};
//!
//! // Save a bitset
//! let file = BitsetFile::new("20240101", "<< 404684003", bitset);
//! file.save("constraint.eclb")?;
//!
//! // Load it back
//! let loaded = BitsetFile::load("constraint.eclb", registry)?;
//! ```

mod manifest;

pub use manifest::{BitsetEntry, BitsetManifest, ConstraintType};

use crate::bitset::{ConceptBitSet, ConceptIdRegistry};
use crate::error::{OptimizerError, OptimizerResult};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::Arc;

/// Magic bytes for bitset files.
const BITSET_MAGIC: &[u8; 4] = b"ECLB";

/// Current bitset file format version.
const BITSET_VERSION: u32 = 1;

/// A serialized bitset file with metadata.
#[derive(Debug)]
pub struct BitsetFile {
    /// SNOMED CT release identifier.
    pub snomed_release: String,
    /// Original ECL expression.
    pub ecl_expression: String,
    /// SHA-256 hash of the ECL expression.
    pub ecl_hash: [u8; 32],
    /// Number of concepts in the bitset.
    pub concept_count: u64,
    /// Serialized bitmap data.
    bitmap_data: Vec<u8>,
}

impl BitsetFile {
    /// Creates a new bitset file from a bitset.
    pub fn new(snomed_release: &str, ecl_expression: &str, bitset: &ConceptBitSet) -> Self {
        let ecl_hash = Self::hash_ecl(ecl_expression);
        let concept_count = bitset.len() as u64;
        let bitmap_data = bitset.serialize();

        Self {
            snomed_release: snomed_release.to_string(),
            ecl_expression: ecl_expression.to_string(),
            ecl_hash,
            concept_count,
            bitmap_data,
        }
    }

    /// Computes SHA-256 hash of an ECL expression.
    fn hash_ecl(ecl: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(ecl.as_bytes());
        hasher.finalize().into()
    }

    /// Saves the bitset to a file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> OptimizerResult<()> {
        let path = path.as_ref();
        let file = File::create(path).map_err(|e| OptimizerError::io_error(path, e))?;
        let mut writer = BufWriter::new(file);

        // Write magic bytes
        writer
            .write_all(BITSET_MAGIC)
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Write version
        writer
            .write_all(&BITSET_VERSION.to_le_bytes())
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Write SNOMED release (length-prefixed)
        let release_bytes = self.snomed_release.as_bytes();
        writer
            .write_all(&(release_bytes.len() as u32).to_le_bytes())
            .map_err(|e| OptimizerError::io_error(path, e))?;
        writer
            .write_all(release_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Write ECL expression (length-prefixed)
        let ecl_bytes = self.ecl_expression.as_bytes();
        writer
            .write_all(&(ecl_bytes.len() as u32).to_le_bytes())
            .map_err(|e| OptimizerError::io_error(path, e))?;
        writer
            .write_all(ecl_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Write ECL hash
        writer
            .write_all(&self.ecl_hash)
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Write concept count
        writer
            .write_all(&self.concept_count.to_le_bytes())
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Write bitmap data (length-prefixed)
        writer
            .write_all(&(self.bitmap_data.len() as u32).to_le_bytes())
            .map_err(|e| OptimizerError::io_error(path, e))?;
        writer
            .write_all(&self.bitmap_data)
            .map_err(|e| OptimizerError::io_error(path, e))?;

        writer
            .flush()
            .map_err(|e| OptimizerError::io_error(path, e))?;

        Ok(())
    }

    /// Loads a bitset file and deserializes to a ConceptBitSet.
    pub fn load<P: AsRef<Path>>(
        path: P,
        registry: Arc<ConceptIdRegistry>,
    ) -> OptimizerResult<(Self, ConceptBitSet)> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| OptimizerError::io_error(path, e))?;
        let mut reader = BufReader::new(file);

        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        reader
            .read_exact(&mut magic)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        if &magic != BITSET_MAGIC {
            return Err(OptimizerError::invalid_format("Invalid magic bytes"));
        }

        // Read and verify version
        let mut version_bytes = [0u8; 4];
        reader
            .read_exact(&mut version_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        let version = u32::from_le_bytes(version_bytes);
        if version != BITSET_VERSION {
            return Err(OptimizerError::invalid_format(format!(
                "Unsupported version: {} (expected {})",
                version, BITSET_VERSION
            )));
        }

        // Read SNOMED release
        let mut len_bytes = [0u8; 4];
        reader
            .read_exact(&mut len_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        let release_len = u32::from_le_bytes(len_bytes) as usize;
        let mut release_bytes = vec![0u8; release_len];
        reader
            .read_exact(&mut release_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        let snomed_release = String::from_utf8_lossy(&release_bytes).to_string();

        // Read ECL expression
        reader
            .read_exact(&mut len_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        let ecl_len = u32::from_le_bytes(len_bytes) as usize;
        let mut ecl_bytes = vec![0u8; ecl_len];
        reader
            .read_exact(&mut ecl_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        let ecl_expression = String::from_utf8_lossy(&ecl_bytes).to_string();

        // Read ECL hash
        let mut ecl_hash = [0u8; 32];
        reader
            .read_exact(&mut ecl_hash)
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Verify ECL hash
        let computed_hash = Self::hash_ecl(&ecl_expression);
        if computed_hash != ecl_hash {
            return Err(OptimizerError::HashMismatch {
                expected: hex::encode(&ecl_hash),
                actual: hex::encode(&computed_hash),
            });
        }

        // Read concept count
        let mut count_bytes = [0u8; 8];
        reader
            .read_exact(&mut count_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        let concept_count = u64::from_le_bytes(count_bytes);

        // Read bitmap data
        reader
            .read_exact(&mut len_bytes)
            .map_err(|e| OptimizerError::io_error(path, e))?;
        let bitmap_len = u32::from_le_bytes(len_bytes) as usize;
        let mut bitmap_data = vec![0u8; bitmap_len];
        reader
            .read_exact(&mut bitmap_data)
            .map_err(|e| OptimizerError::io_error(path, e))?;

        // Deserialize bitset
        let bitset = ConceptBitSet::deserialize(&bitmap_data, registry)
            .map_err(|e| OptimizerError::DeserializationError(e))?;

        // Verify concept count
        if bitset.len() as u64 != concept_count {
            return Err(OptimizerError::invalid_format(format!(
                "Concept count mismatch: expected {}, got {}",
                concept_count,
                bitset.len()
            )));
        }

        let metadata = Self {
            snomed_release,
            ecl_expression,
            ecl_hash,
            concept_count,
            bitmap_data,
        };

        Ok((metadata, bitset))
    }

    /// Returns the ECL hash as a hex string.
    pub fn ecl_hash_hex(&self) -> String {
        hex::encode(&self.ecl_hash)
    }

    /// Validates that a file matches the expected ECL expression.
    pub fn validate_ecl(&self, expected_ecl: &str) -> bool {
        let expected_hash = Self::hash_ecl(expected_ecl);
        self.ecl_hash == expected_hash
    }
}

/// Helper module for hex encoding (minimal implementation).
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::tempdir;

    fn create_test_registry() -> Arc<ConceptIdRegistry> {
        Arc::new(ConceptIdRegistry::from_concepts(
            vec![100u64, 200, 300, 400, 500].into_iter(),
        ))
    }

    fn create_test_bitset(registry: Arc<ConceptIdRegistry>) -> ConceptBitSet {
        let ids: HashSet<u64> = [100, 200, 300].into_iter().collect();
        ConceptBitSet::from_hash_set(&ids, registry)
    }

    #[test]
    fn test_save_and_load() {
        let registry = create_test_registry();
        let bitset = create_test_bitset(registry.clone());

        let file = BitsetFile::new("20240101", "<< 404684003", &bitset);

        // Save to temp file
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.eclb");
        file.save(&path).unwrap();

        // Load back
        let (loaded_meta, loaded_bitset) = BitsetFile::load(&path, registry).unwrap();

        assert_eq!(loaded_meta.snomed_release, "20240101");
        assert_eq!(loaded_meta.ecl_expression, "<< 404684003");
        assert_eq!(loaded_meta.concept_count, 3);
        assert_eq!(loaded_bitset.len(), 3);
        assert!(loaded_bitset.contains(100));
        assert!(loaded_bitset.contains(200));
        assert!(loaded_bitset.contains(300));
    }

    #[test]
    fn test_ecl_hash_validation() {
        let registry = create_test_registry();
        let bitset = create_test_bitset(registry);

        let file = BitsetFile::new("20240101", "<< 404684003", &bitset);

        assert!(file.validate_ecl("<< 404684003"));
        assert!(!file.validate_ecl("<< 73211009"));
    }

    #[test]
    fn test_invalid_magic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.eclb");

        // Write invalid file
        std::fs::write(&path, b"BAAD").unwrap();

        let registry = create_test_registry();
        let result = BitsetFile::load(&path, registry);
        assert!(result.is_err());
    }
}
