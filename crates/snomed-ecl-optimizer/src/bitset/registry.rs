//! Concept ID registry for mapping between SctId (u64) and compact indices (u32).

use snomed_ecl::SctId;
use std::collections::HashMap;

/// Registry that maps between SctId (u64) and compact indices (u32).
///
/// SNOMED CT IDs are 64-bit but sparse. Roaring Bitmaps use 32-bit indices.
/// This registry creates a dense mapping to u32 indices suitable for
/// roaring bitmaps.
///
/// The registry is immutable after construction to ensure consistency
/// between bitsets using the same registry.
///
/// # Example
///
/// ```ignore
/// use snomed_ecl_optimizer::bitset::ConceptIdRegistry;
///
/// // Create from concept IDs
/// let registry = ConceptIdRegistry::from_concepts(
///     store.all_concept_ids()
/// );
///
/// // Map between SctId and index
/// let idx = registry.get_index(73211009).unwrap();
/// let id = registry.get_concept_id(idx).unwrap();
/// assert_eq!(id, 73211009);
/// ```
#[derive(Clone)]
pub struct ConceptIdRegistry {
    /// SctId -> u32 index mapping.
    id_to_index: HashMap<SctId, u32>,
    /// u32 index -> SctId mapping.
    index_to_id: Vec<SctId>,
}

impl ConceptIdRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            id_to_index: HashMap::new(),
            index_to_id: Vec::new(),
        }
    }

    /// Creates a registry with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            id_to_index: HashMap::with_capacity(capacity),
            index_to_id: Vec::with_capacity(capacity),
        }
    }

    /// Creates a registry from an iterator of concept IDs.
    ///
    /// Each unique concept ID is assigned a sequential index starting from 0.
    pub fn from_concepts<I: IntoIterator<Item = SctId>>(concepts: I) -> Self {
        let mut registry = Self::new();
        for id in concepts {
            registry.register(id);
        }
        registry
    }

    /// Registers a concept ID and returns its index.
    ///
    /// If the ID is already registered, returns the existing index.
    pub fn register(&mut self, id: SctId) -> u32 {
        if let Some(&idx) = self.id_to_index.get(&id) {
            return idx;
        }
        let idx = self.index_to_id.len() as u32;
        self.id_to_index.insert(id, idx);
        self.index_to_id.push(id);
        idx
    }

    /// Gets the compact index for a concept ID.
    ///
    /// Returns `None` if the concept ID is not registered.
    #[inline]
    pub fn get_index(&self, id: SctId) -> Option<u32> {
        self.id_to_index.get(&id).copied()
    }

    /// Gets the concept ID for a compact index.
    ///
    /// Returns `None` if the index is out of bounds.
    #[inline]
    pub fn get_concept_id(&self, index: u32) -> Option<SctId> {
        self.index_to_id.get(index as usize).copied()
    }

    /// Returns the number of registered concepts.
    #[inline]
    pub fn len(&self) -> usize {
        self.index_to_id.len()
    }

    /// Returns true if the registry is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index_to_id.is_empty()
    }

    /// Returns true if the concept ID is registered.
    #[inline]
    pub fn contains(&self, id: SctId) -> bool {
        self.id_to_index.contains_key(&id)
    }

    /// Returns an iterator over all registered concept IDs.
    pub fn concept_ids(&self) -> impl Iterator<Item = SctId> + '_ {
        self.index_to_id.iter().copied()
    }

    /// Returns an iterator over (index, concept_id) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (u32, SctId)> + '_ {
        self.index_to_id
            .iter()
            .enumerate()
            .map(|(i, &id)| (i as u32, id))
    }

    /// Converts a slice of concept IDs to their indices.
    ///
    /// Unknown IDs are filtered out.
    pub fn to_indices(&self, ids: &[SctId]) -> Vec<u32> {
        ids.iter().filter_map(|&id| self.get_index(id)).collect()
    }

    /// Converts a slice of indices back to concept IDs.
    ///
    /// Invalid indices are filtered out.
    pub fn to_concept_ids(&self, indices: &[u32]) -> Vec<SctId> {
        indices
            .iter()
            .filter_map(|&idx| self.get_concept_id(idx))
            .collect()
    }

    /// Returns estimated memory usage in bytes.
    pub fn memory_size(&self) -> usize {
        let hashmap_size = self.id_to_index.capacity() * (8 + 4 + 8); // key + value + overhead
        let vec_size = self.index_to_id.capacity() * 8;
        hashmap_size + vec_size + std::mem::size_of::<Self>()
    }
}

impl Default for ConceptIdRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ConceptIdRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConceptIdRegistry")
            .field("len", &self.len())
            .field("memory_size", &self.memory_size())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let registry = ConceptIdRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_from_concepts() {
        let concepts = vec![100u64, 200, 300, 400];
        let registry = ConceptIdRegistry::from_concepts(concepts.clone().into_iter());

        assert_eq!(registry.len(), 4);
        for id in concepts {
            assert!(registry.contains(id));
        }
    }

    #[test]
    fn test_register_and_lookup() {
        let mut registry = ConceptIdRegistry::new();

        let idx1 = registry.register(100);
        let idx2 = registry.register(200);
        let idx3 = registry.register(100); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as first registration

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.get_index(100), Some(0));
        assert_eq!(registry.get_index(200), Some(1));
        assert_eq!(registry.get_index(300), None);

        assert_eq!(registry.get_concept_id(0), Some(100));
        assert_eq!(registry.get_concept_id(1), Some(200));
        assert_eq!(registry.get_concept_id(2), None);
    }

    #[test]
    fn test_contains() {
        let registry = ConceptIdRegistry::from_concepts(vec![100u64, 200, 300].into_iter());

        assert!(registry.contains(100));
        assert!(registry.contains(200));
        assert!(registry.contains(300));
        assert!(!registry.contains(400));
    }

    #[test]
    fn test_iter() {
        let concepts = vec![100u64, 200, 300];
        let registry = ConceptIdRegistry::from_concepts(concepts.clone().into_iter());

        let collected: Vec<(u32, SctId)> = registry.iter().collect();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], (0, 100));
        assert_eq!(collected[1], (1, 200));
        assert_eq!(collected[2], (2, 300));
    }

    #[test]
    fn test_concept_ids() {
        let concepts = vec![100u64, 200, 300];
        let registry = ConceptIdRegistry::from_concepts(concepts.clone().into_iter());

        let collected: Vec<SctId> = registry.concept_ids().collect();
        assert_eq!(collected, concepts);
    }

    #[test]
    fn test_to_indices() {
        let registry = ConceptIdRegistry::from_concepts(vec![100u64, 200, 300].into_iter());

        let indices = registry.to_indices(&[100, 300, 400]);
        assert_eq!(indices, vec![0, 2]); // 400 is filtered out
    }

    #[test]
    fn test_to_concept_ids() {
        let registry = ConceptIdRegistry::from_concepts(vec![100u64, 200, 300].into_iter());

        let ids = registry.to_concept_ids(&[0, 2, 5]);
        assert_eq!(ids, vec![100, 300]); // Index 5 is filtered out
    }

    #[test]
    fn test_large_registry() {
        // Test with realistic SNOMED-like IDs
        let concepts: Vec<SctId> = (0..10000).map(|i| 100000000 + i).collect();
        let registry = ConceptIdRegistry::from_concepts(concepts.clone().into_iter());

        assert_eq!(registry.len(), 10000);

        // All lookups should work
        for (i, &id) in concepts.iter().enumerate() {
            assert_eq!(registry.get_index(id), Some(i as u32));
            assert_eq!(registry.get_concept_id(i as u32), Some(id));
        }
    }
}
