//! Deduplication strategies for wordlist processing
//!
//! Provides multiple deduplication approaches optimized for different data sizes:
//! - Memory: Fast in-memory HashSet (for datasets that fit in RAM)
//! - Bloom: Probabilistic bloom filter (for very large datasets with acceptable false positive rate)
//! - Disk: RocksDB-based disk storage (for unlimited size, but slower)

use ahash::RandomState;
use hashbrown::HashSet;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// Statistics for deduplication operations
#[derive(Debug, Default)]
pub struct DedupStats {
    /// Total items processed
    pub total_processed: AtomicU64,
    /// Unique items found
    pub unique_count: AtomicU64,
    /// Duplicate items found
    pub duplicate_count: AtomicU64,
}

impl DedupStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_unique(&self) {
        self.total_processed.fetch_add(1, Ordering::Relaxed);
        self.unique_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_duplicate(&self) {
        self.total_processed.fetch_add(1, Ordering::Relaxed);
        self.duplicate_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_total(&self) -> u64 {
        self.total_processed.load(Ordering::Relaxed)
    }
    
    pub fn get_unique(&self) -> u64 {
        self.unique_count.load(Ordering::Relaxed)
    }
    
    pub fn get_duplicates(&self) -> u64 {
        self.duplicate_count.load(Ordering::Relaxed)
    }
}

/// Trait for deduplication implementations
pub trait Deduplicator: Send + Sync {
    /// Check if item is unique and add it if so
    /// Returns true if the item is unique (not seen before)
    fn insert(&self, item: &str) -> bool;
    
    /// Check if item exists without adding it
    fn contains(&self, item: &str) -> bool;
    
    /// Get the number of unique items
    fn len(&self) -> usize;
    
    /// Check if empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Clear all items
    fn clear(&self);
    
    /// Get approximate memory usage in bytes
    fn memory_usage(&self) -> usize;
}

/// In-memory HashSet-based deduplicator
/// 
/// Fastest option but requires enough RAM to hold all unique items.
pub struct MemoryDeduplicator {
    set: RwLock<HashSet<String, RandomState>>,
    hasher: RandomState,
}

impl MemoryDeduplicator {
    pub fn new() -> Self {
        Self {
            set: RwLock::new(HashSet::with_hasher(RandomState::new())),
            hasher: RandomState::new(),
        }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            set: RwLock::new(HashSet::with_capacity_and_hasher(capacity, RandomState::new())),
            hasher: RandomState::new(),
        }
    }
}

impl Default for MemoryDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Deduplicator for MemoryDeduplicator {
    fn insert(&self, item: &str) -> bool {
        let mut set = self.set.write().unwrap();
        set.insert(item.to_string())
    }
    
    fn contains(&self, item: &str) -> bool {
        let set = self.set.read().unwrap();
        set.contains(item)
    }
    
    fn len(&self) -> usize {
        let set = self.set.read().unwrap();
        set.len()
    }
    
    fn clear(&self) {
        let mut set = self.set.write().unwrap();
        set.clear();
    }
    
    fn memory_usage(&self) -> usize {
        let set = self.set.read().unwrap();
        // Approximate: each entry is roughly String overhead + content + HashSet overhead
        set.len() * 64 + set.capacity() * 8
    }
}

/// Bloom filter-based deduplicator
///
/// Uses much less memory than HashSet but has a small false positive rate.
/// False positives mean some unique items might be incorrectly marked as duplicates.
pub struct BloomDeduplicator {
    bits: Vec<AtomicU64>,
    num_hashes: usize,
    hasher: RandomState,
    estimated_count: AtomicU64,
}

impl BloomDeduplicator {
    /// Create a new bloom filter
    /// 
    /// # Arguments
    /// * `expected_items` - Expected number of unique items
    /// * `false_positive_rate` - Desired false positive rate (e.g., 0.001 for 0.1%)
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        // Calculate optimal size and number of hash functions
        let ln2 = std::f64::consts::LN_2;
        let ln2_squared = ln2 * ln2;
        
        // m = -n * ln(p) / (ln(2)^2)
        let num_bits = (-(expected_items as f64) * false_positive_rate.ln() / ln2_squared).ceil() as usize;
        let num_bits = num_bits.max(64); // Minimum 64 bits
        
        // k = (m/n) * ln(2)
        let num_hashes = ((num_bits as f64 / expected_items as f64) * ln2).ceil() as usize;
        let num_hashes = num_hashes.clamp(1, 16); // Between 1 and 16 hash functions
        
        // Round up to multiple of 64 for AtomicU64
        let num_u64s = (num_bits + 63) / 64;
        
        let bits = (0..num_u64s).map(|_| AtomicU64::new(0)).collect();
        
        Self {
            bits,
            num_hashes,
            hasher: RandomState::new(),
            estimated_count: AtomicU64::new(0),
        }
    }
    
    /// Create with specific parameters
    pub fn with_params(num_bits: usize, num_hashes: usize) -> Self {
        let num_u64s = (num_bits + 63) / 64;
        let bits = (0..num_u64s).map(|_| AtomicU64::new(0)).collect();
        
        Self {
            bits,
            num_hashes,
            hasher: RandomState::new(),
            estimated_count: AtomicU64::new(0),
        }
    }
    
    fn get_hash_indices(&self, item: &str) -> Vec<usize> {
        let num_bits = self.bits.len() * 64;
        let mut indices = Vec::with_capacity(self.num_hashes);
        
        // Use double hashing technique
        let mut hasher1 = self.hasher.build_hasher();
        item.hash(&mut hasher1);
        let h1 = hasher1.finish() as usize;
        
        let mut hasher2 = self.hasher.build_hasher();
        hasher2.write_usize(h1);
        item.hash(&mut hasher2);
        let h2 = hasher2.finish() as usize;
        
        for i in 0..self.num_hashes {
            let index = (h1.wrapping_add(i.wrapping_mul(h2))) % num_bits;
            indices.push(index);
        }
        
        indices
    }
    
    fn set_bit(&self, index: usize) -> bool {
        let u64_index = index / 64;
        let bit_index = index % 64;
        let mask = 1u64 << bit_index;
        
        let old = self.bits[u64_index].fetch_or(mask, Ordering::Relaxed);
        (old & mask) == 0 // Return true if bit was not set before
    }
    
    fn get_bit(&self, index: usize) -> bool {
        let u64_index = index / 64;
        let bit_index = index % 64;
        let mask = 1u64 << bit_index;
        
        (self.bits[u64_index].load(Ordering::Relaxed) & mask) != 0
    }
}

impl Deduplicator for BloomDeduplicator {
    fn insert(&self, item: &str) -> bool {
        let indices = self.get_hash_indices(item);
        
        // Check if all bits are already set (probable duplicate)
        let probably_exists = indices.iter().all(|&i| self.get_bit(i));
        
        if probably_exists {
            return false;
        }
        
        // Set all bits
        let mut any_new = false;
        for index in indices {
            if self.set_bit(index) {
                any_new = true;
            }
        }
        
        if any_new {
            self.estimated_count.fetch_add(1, Ordering::Relaxed);
        }
        
        !probably_exists
    }
    
    fn contains(&self, item: &str) -> bool {
        let indices = self.get_hash_indices(item);
        indices.iter().all(|&i| self.get_bit(i))
    }
    
    fn len(&self) -> usize {
        self.estimated_count.load(Ordering::Relaxed) as usize
    }
    
    fn clear(&self) {
        for bit in &self.bits {
            bit.store(0, Ordering::Relaxed);
        }
        self.estimated_count.store(0, Ordering::Relaxed);
    }
    
    fn memory_usage(&self) -> usize {
        self.bits.len() * 8
    }
}

/// Sharded memory deduplicator for better parallel performance
pub struct ShardedDeduplicator {
    shards: Vec<RwLock<HashSet<String, RandomState>>>,
    hasher: RandomState,
}

impl ShardedDeduplicator {
    pub fn new(num_shards: usize) -> Self {
        let shards = (0..num_shards)
            .map(|_| RwLock::new(HashSet::with_hasher(RandomState::new())))
            .collect();
        
        Self {
            shards,
            hasher: RandomState::new(),
        }
    }
    
    pub fn with_capacity(num_shards: usize, capacity_per_shard: usize) -> Self {
        let shards = (0..num_shards)
            .map(|_| RwLock::new(HashSet::with_capacity_and_hasher(capacity_per_shard, RandomState::new())))
            .collect();
        
        Self {
            shards,
            hasher: RandomState::new(),
        }
    }
    
    fn get_shard_index(&self, item: &str) -> usize {
        let mut hasher = self.hasher.build_hasher();
        item.hash(&mut hasher);
        hasher.finish() as usize % self.shards.len()
    }
}

impl Deduplicator for ShardedDeduplicator {
    fn insert(&self, item: &str) -> bool {
        let shard_idx = self.get_shard_index(item);
        let mut shard = self.shards[shard_idx].write().unwrap();
        shard.insert(item.to_string())
    }
    
    fn contains(&self, item: &str) -> bool {
        let shard_idx = self.get_shard_index(item);
        let shard = self.shards[shard_idx].read().unwrap();
        shard.contains(item)
    }
    
    fn len(&self) -> usize {
        self.shards.iter()
            .map(|s| s.read().unwrap().len())
            .sum()
    }
    
    fn clear(&self) {
        for shard in &self.shards {
            shard.write().unwrap().clear();
        }
    }
    
    fn memory_usage(&self) -> usize {
        self.shards.iter()
            .map(|s| {
                let set = s.read().unwrap();
                set.len() * 64 + set.capacity() * 8
            })
            .sum()
    }
}

/// No-op deduplicator for when deduplication is disabled
pub struct NoOpDeduplicator {
    count: AtomicU64,
}

impl NoOpDeduplicator {
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
        }
    }
}

impl Default for NoOpDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Deduplicator for NoOpDeduplicator {
    fn insert(&self, _item: &str) -> bool {
        self.count.fetch_add(1, Ordering::Relaxed);
        true // Always "unique" since we don't track
    }
    
    fn contains(&self, _item: &str) -> bool {
        false // Never contains anything
    }
    
    fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed) as usize
    }
    
    fn clear(&self) {
        self.count.store(0, Ordering::Relaxed);
    }
    
    fn memory_usage(&self) -> usize {
        8 // Just the counter
    }
}

/// Factory for creating deduplicators based on configuration
pub fn create_deduplicator(
    strategy: crate::cli::DedupStrategy,
    expected_items: usize,
    memory_limit: usize,
) -> Box<dyn Deduplicator> {
    match strategy {
        crate::cli::DedupStrategy::Memory => {
            // Use sharded deduplicator for parallel performance
            let num_shards = num_cpus::get() * 4;
            let capacity_per_shard = expected_items / num_shards;
            Box::new(ShardedDeduplicator::with_capacity(num_shards, capacity_per_shard))
        }
        crate::cli::DedupStrategy::Bloom => {
            // Use bloom filter with 0.1% false positive rate
            Box::new(BloomDeduplicator::new(expected_items, 0.001))
        }
        #[cfg(feature = "disk-dedup")]
        crate::cli::DedupStrategy::Disk => {
            // Disk-based deduplication would be implemented here
            unimplemented!("Disk-based deduplication requires the 'disk-dedup' feature")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_deduplicator() {
        let dedup = MemoryDeduplicator::new();
        
        assert!(dedup.insert("test1"));
        assert!(dedup.insert("test2"));
        assert!(!dedup.insert("test1")); // Duplicate
        
        assert_eq!(dedup.len(), 2);
        assert!(dedup.contains("test1"));
        assert!(!dedup.contains("test3"));
    }
    
    #[test]
    fn test_bloom_deduplicator() {
        let dedup = BloomDeduplicator::new(1000, 0.01);
        
        assert!(dedup.insert("test1"));
        assert!(dedup.insert("test2"));
        assert!(!dedup.insert("test1")); // Should detect duplicate
        
        assert!(dedup.contains("test1"));
        assert!(dedup.contains("test2"));
    }
    
    #[test]
    fn test_sharded_deduplicator() {
        let dedup = ShardedDeduplicator::new(4);
        
        assert!(dedup.insert("test1"));
        assert!(dedup.insert("test2"));
        assert!(dedup.insert("test3"));
        assert!(!dedup.insert("test1"));
        
        assert_eq!(dedup.len(), 3);
    }
    
    #[test]
    fn test_noop_deduplicator() {
        let dedup = NoOpDeduplicator::new();
        
        assert!(dedup.insert("test1"));
        assert!(dedup.insert("test1")); // Always unique
        
        assert_eq!(dedup.len(), 2);
        assert!(!dedup.contains("test1")); // Never contains
    }
}
