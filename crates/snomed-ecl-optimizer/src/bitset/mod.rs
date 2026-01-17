//! Roaring bitmap-based concept sets for memory-efficient storage.
//!
//! This module provides efficient storage and set operations for concept IDs
//! using Roaring Bitmaps, which offer excellent compression for sparse/dense sets.
//!
//! # Example
//!
//! ```ignore
//! use snomed_ecl_optimizer::bitset::{ConceptBitSet, ConceptIdRegistry};
//! use std::sync::Arc;
//!
//! // Create a registry from all concepts
//! let registry = Arc::new(ConceptIdRegistry::from_concepts(
//!     store.all_concept_ids()
//! ));
//!
//! // Create bitsets from concept sets
//! let set1 = ConceptBitSet::from_hash_set(&descendants1, registry.clone());
//! let set2 = ConceptBitSet::from_hash_set(&descendants2, registry.clone());
//!
//! // Efficient set operations
//! let intersection = set1.intersection(&set2);  // AND
//! let union = set1.union(&set2);                // OR
//! let difference = set1.difference(&set2);      // MINUS
//!
//! // Check membership
//! if intersection.contains(concept_id) {
//!     println!("Concept is in both sets");
//! }
//! ```

mod registry;
// mod evaluator;

pub use registry::ConceptIdRegistry;
// pub use evaluator::BitsetEvaluator;

use roaring::RoaringBitmap;
use snomed_ecl::SctId;
use std::collections::HashSet;
use std::sync::Arc;

/// A set of concepts stored as a Roaring Bitmap.
///
/// Provides efficient set operations (AND, OR, MINUS) with
/// compression for sparse/dense concept sets.
///
/// Roaring Bitmaps use u32 indices internally, so this type uses
/// a [`ConceptIdRegistry`] to map between SctId (u64) and u32 indices.
#[derive(Clone)]
pub struct ConceptBitSet {
    bitmap: RoaringBitmap,
    registry: Arc<ConceptIdRegistry>,
}

impl ConceptBitSet {
    /// Creates a new empty bitset.
    pub fn new(registry: Arc<ConceptIdRegistry>) -> Self {
        Self {
            bitmap: RoaringBitmap::new(),
            registry,
        }
    }

    /// Creates a bitset from a HashSet of concept IDs.
    ///
    /// Concepts not in the registry are silently ignored.
    pub fn from_hash_set(ids: &HashSet<SctId>, registry: Arc<ConceptIdRegistry>) -> Self {
        let mut bitmap = RoaringBitmap::new();
        for &id in ids {
            if let Some(idx) = registry.get_index(id) {
                bitmap.insert(idx);
            }
        }
        Self { bitmap, registry }
    }

    /// Creates a bitset from an iterator of concept IDs.
    ///
    /// Concepts not in the registry are silently ignored.
    pub fn from_iter<I: IntoIterator<Item = SctId>>(
        ids: I,
        registry: Arc<ConceptIdRegistry>,
    ) -> Self {
        let mut bitmap = RoaringBitmap::new();
        for id in ids {
            if let Some(idx) = registry.get_index(id) {
                bitmap.insert(idx);
            }
        }
        Self { bitmap, registry }
    }

    /// Returns a reference to the underlying registry.
    pub fn registry(&self) -> &Arc<ConceptIdRegistry> {
        &self.registry
    }

    /// Inserts a concept ID into the bitset.
    ///
    /// Returns `true` if the concept was newly inserted, `false` if it was already present.
    /// Returns `false` if the concept is not in the registry.
    pub fn insert(&mut self, concept_id: SctId) -> bool {
        if let Some(idx) = self.registry.get_index(concept_id) {
            self.bitmap.insert(idx)
        } else {
            false
        }
    }

    /// Removes a concept ID from the bitset.
    ///
    /// Returns `true` if the concept was present, `false` otherwise.
    pub fn remove(&mut self, concept_id: SctId) -> bool {
        if let Some(idx) = self.registry.get_index(concept_id) {
            self.bitmap.remove(idx)
        } else {
            false
        }
    }

    /// Checks if a concept is in the set (O(1)).
    #[inline]
    pub fn contains(&self, concept_id: SctId) -> bool {
        self.registry
            .get_index(concept_id)
            .is_some_and(|idx| self.bitmap.contains(idx))
    }

    /// Returns the number of concepts in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.bitmap.len() as usize
    }

    /// Returns true if the set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bitmap.is_empty()
    }

    /// Computes intersection (AND) - returns a new bitset.
    ///
    /// # Panics
    ///
    /// Panics if the two bitsets use different registries.
    pub fn intersection(&self, other: &Self) -> Self {
        assert!(
            Arc::ptr_eq(&self.registry, &other.registry),
            "Cannot intersect bitsets with different registries"
        );
        Self {
            bitmap: &self.bitmap & &other.bitmap,
            registry: self.registry.clone(),
        }
    }

    /// Computes union (OR) - returns a new bitset.
    ///
    /// # Panics
    ///
    /// Panics if the two bitsets use different registries.
    pub fn union(&self, other: &Self) -> Self {
        assert!(
            Arc::ptr_eq(&self.registry, &other.registry),
            "Cannot union bitsets with different registries"
        );
        Self {
            bitmap: &self.bitmap | &other.bitmap,
            registry: self.registry.clone(),
        }
    }

    /// Computes difference (MINUS) - returns a new bitset.
    ///
    /// # Panics
    ///
    /// Panics if the two bitsets use different registries.
    pub fn difference(&self, other: &Self) -> Self {
        assert!(
            Arc::ptr_eq(&self.registry, &other.registry),
            "Cannot difference bitsets with different registries"
        );
        Self {
            bitmap: &self.bitmap - &other.bitmap,
            registry: self.registry.clone(),
        }
    }

    /// Computes intersection in-place (modifies self).
    ///
    /// # Panics
    ///
    /// Panics if the two bitsets use different registries.
    pub fn and_inplace(&mut self, other: &Self) {
        assert!(
            Arc::ptr_eq(&self.registry, &other.registry),
            "Cannot intersect bitsets with different registries"
        );
        self.bitmap &= &other.bitmap;
    }

    /// Computes union in-place (modifies self).
    ///
    /// # Panics
    ///
    /// Panics if the two bitsets use different registries.
    pub fn or_inplace(&mut self, other: &Self) {
        assert!(
            Arc::ptr_eq(&self.registry, &other.registry),
            "Cannot union bitsets with different registries"
        );
        self.bitmap |= &other.bitmap;
    }

    /// Computes difference in-place (modifies self).
    ///
    /// # Panics
    ///
    /// Panics if the two bitsets use different registries.
    pub fn andnot_inplace(&mut self, other: &Self) {
        assert!(
            Arc::ptr_eq(&self.registry, &other.registry),
            "Cannot difference bitsets with different registries"
        );
        self.bitmap -= &other.bitmap;
    }

    /// Converts to a HashSet of concept IDs.
    pub fn to_hash_set(&self) -> HashSet<SctId> {
        self.bitmap
            .iter()
            .filter_map(|idx| self.registry.get_concept_id(idx))
            .collect()
    }

    /// Returns an iterator over concept IDs in the set.
    pub fn iter(&self) -> impl Iterator<Item = SctId> + '_ {
        self.bitmap
            .iter()
            .filter_map(|idx| self.registry.get_concept_id(idx))
    }

    /// Filters a slice of candidates, returning only those in the bitset.
    pub fn filter(&self, candidates: &[SctId]) -> Vec<SctId> {
        candidates
            .iter()
            .copied()
            .filter(|&id| self.contains(id))
            .collect()
    }

    /// Returns the serialized size in bytes.
    pub fn serialized_size(&self) -> usize {
        self.bitmap.serialized_size()
    }

    /// Returns approximate memory usage in bytes.
    pub fn memory_size(&self) -> usize {
        self.serialized_size() + std::mem::size_of::<Self>()
    }

    /// Serializes the bitmap to a byte vector.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.bitmap
            .serialize_into(&mut buf)
            .expect("serialization to vec should not fail");
        buf
    }

    /// Deserializes a bitmap from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn deserialize(bytes: &[u8], registry: Arc<ConceptIdRegistry>) -> Result<Self, String> {
        let bitmap = RoaringBitmap::deserialize_from(bytes)
            .map_err(|e| format!("Failed to deserialize bitmap: {}", e))?;
        Ok(Self { bitmap, registry })
    }

    /// Returns a reference to the underlying bitmap (for advanced use).
    pub fn as_bitmap(&self) -> &RoaringBitmap {
        &self.bitmap
    }
}

impl std::fmt::Debug for ConceptBitSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConceptBitSet")
            .field("len", &self.len())
            .field("serialized_size", &self.serialized_size())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> Arc<ConceptIdRegistry> {
        Arc::new(ConceptIdRegistry::from_concepts(
            vec![100u64, 200, 300, 400, 500, 600].into_iter(),
        ))
    }

    #[test]
    fn test_new_empty() {
        let registry = create_test_registry();
        let bitset = ConceptBitSet::new(registry);

        assert!(bitset.is_empty());
        assert_eq!(bitset.len(), 0);
    }

    #[test]
    fn test_from_hash_set() {
        let registry = create_test_registry();
        let ids: HashSet<SctId> = [100, 200, 300].into_iter().collect();
        let bitset = ConceptBitSet::from_hash_set(&ids, registry);

        assert_eq!(bitset.len(), 3);
        assert!(bitset.contains(100));
        assert!(bitset.contains(200));
        assert!(bitset.contains(300));
        assert!(!bitset.contains(400));
    }

    #[test]
    fn test_insert_remove() {
        let registry = create_test_registry();
        let mut bitset = ConceptBitSet::new(registry);

        assert!(bitset.insert(100));
        assert!(bitset.contains(100));
        assert_eq!(bitset.len(), 1);

        // Insert again - should return false
        assert!(!bitset.insert(100));
        assert_eq!(bitset.len(), 1);

        // Remove
        assert!(bitset.remove(100));
        assert!(!bitset.contains(100));
        assert_eq!(bitset.len(), 0);

        // Remove again - should return false
        assert!(!bitset.remove(100));
    }

    #[test]
    fn test_intersection() {
        let registry = create_test_registry();
        let set1 = ConceptBitSet::from_hash_set(&[100, 200, 300].into_iter().collect(), registry.clone());
        let set2 = ConceptBitSet::from_hash_set(&[200, 300, 400].into_iter().collect(), registry.clone());

        let intersection = set1.intersection(&set2);

        assert_eq!(intersection.len(), 2);
        assert!(!intersection.contains(100));
        assert!(intersection.contains(200));
        assert!(intersection.contains(300));
        assert!(!intersection.contains(400));
    }

    #[test]
    fn test_union() {
        let registry = create_test_registry();
        let set1 = ConceptBitSet::from_hash_set(&[100, 200].into_iter().collect(), registry.clone());
        let set2 = ConceptBitSet::from_hash_set(&[300, 400].into_iter().collect(), registry.clone());

        let union = set1.union(&set2);

        assert_eq!(union.len(), 4);
        assert!(union.contains(100));
        assert!(union.contains(200));
        assert!(union.contains(300));
        assert!(union.contains(400));
    }

    #[test]
    fn test_difference() {
        let registry = create_test_registry();
        let set1 = ConceptBitSet::from_hash_set(&[100, 200, 300].into_iter().collect(), registry.clone());
        let set2 = ConceptBitSet::from_hash_set(&[200, 300, 400].into_iter().collect(), registry.clone());

        let diff = set1.difference(&set2);

        assert_eq!(diff.len(), 1);
        assert!(diff.contains(100));
        assert!(!diff.contains(200));
    }

    #[test]
    fn test_inplace_operations() {
        let registry = create_test_registry();
        let mut set1 = ConceptBitSet::from_hash_set(&[100, 200, 300].into_iter().collect(), registry.clone());
        let set2 = ConceptBitSet::from_hash_set(&[200, 300, 400].into_iter().collect(), registry.clone());

        set1.and_inplace(&set2);
        assert_eq!(set1.len(), 2);
        assert!(set1.contains(200));
        assert!(set1.contains(300));
    }

    #[test]
    fn test_to_hash_set() {
        let registry = create_test_registry();
        let original: HashSet<SctId> = [100, 200, 300].into_iter().collect();
        let bitset = ConceptBitSet::from_hash_set(&original, registry);

        let converted = bitset.to_hash_set();
        assert_eq!(converted, original);
    }

    #[test]
    fn test_iter() {
        let registry = create_test_registry();
        let ids: HashSet<SctId> = [100, 200, 300].into_iter().collect();
        let bitset = ConceptBitSet::from_hash_set(&ids, registry);

        let collected: HashSet<SctId> = bitset.iter().collect();
        assert_eq!(collected, ids);
    }

    #[test]
    fn test_filter() {
        let registry = create_test_registry();
        let bitset = ConceptBitSet::from_hash_set(&[100, 200, 300].into_iter().collect(), registry);

        let candidates = vec![100, 200, 400, 500];
        let filtered = bitset.filter(&candidates);

        assert_eq!(filtered, vec![100, 200]);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let registry = create_test_registry();
        let ids: HashSet<SctId> = [100, 200, 300].into_iter().collect();
        let original = ConceptBitSet::from_hash_set(&ids, registry.clone());

        let bytes = original.serialize();
        let deserialized = ConceptBitSet::deserialize(&bytes, registry).unwrap();

        assert_eq!(deserialized.len(), original.len());
        assert!(deserialized.contains(100));
        assert!(deserialized.contains(200));
        assert!(deserialized.contains(300));
    }

    #[test]
    #[should_panic(expected = "different registries")]
    fn test_intersection_different_registries() {
        let registry1 = Arc::new(ConceptIdRegistry::from_concepts(vec![100u64, 200].into_iter()));
        let registry2 = Arc::new(ConceptIdRegistry::from_concepts(vec![100u64, 200].into_iter()));

        let set1 = ConceptBitSet::from_hash_set(&[100].into_iter().collect(), registry1);
        let set2 = ConceptBitSet::from_hash_set(&[100].into_iter().collect(), registry2);

        let _ = set1.intersection(&set2); // Should panic
    }
}
