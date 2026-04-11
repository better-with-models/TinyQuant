//! Integration tests for `tinyquant_core::codec::RotationCache`.

use tinyquant_core::codec::{RotationCache, ROTATION_CACHE_DEFAULT_CAPACITY};

#[test]
fn default_capacity_matches_python_lru_cache_maxsize() {
    assert_eq!(ROTATION_CACHE_DEFAULT_CAPACITY, 8);
    let cache = RotationCache::default();
    assert_eq!(cache.capacity(), 8);
    assert!(cache.is_empty());
}

#[test]
fn hit_returns_equal_matrix_for_repeated_key() {
    let cache = RotationCache::new(4);
    let a = cache.get_or_build(42, 32);
    let b = cache.get_or_build(42, 32);
    assert_eq!(a.matrix(), b.matrix());
    assert_eq!(cache.len(), 1);
}

#[test]
fn miss_stores_new_entry() {
    let cache = RotationCache::new(4);
    cache.get_or_build(1, 8);
    cache.get_or_build(2, 8);
    cache.get_or_build(3, 16);
    assert_eq!(cache.len(), 3);
}

#[test]
fn evicts_lru_when_capacity_reached() {
    let cache = RotationCache::new(2);
    cache.get_or_build(1, 8);
    cache.get_or_build(2, 8);
    // Third insert must evict seed=1,dim=8 (oldest).
    cache.get_or_build(3, 8);
    assert_eq!(cache.len(), 2);

    // Re-requesting seed=1 is a miss (rebuilds), which evicts seed=2 next.
    cache.get_or_build(1, 8);
    assert_eq!(cache.len(), 2);
    // Now the cache holds {1, 3}; requesting seed=2 is a miss again.
    cache.get_or_build(2, 8);
    assert_eq!(cache.len(), 2);
}

#[test]
fn hit_promotes_touched_entry_to_mru() {
    let cache = RotationCache::new(2);
    cache.get_or_build(1, 8); // LRU, MRU = [1]
    cache.get_or_build(2, 8); // MRU = [2, 1]
    cache.get_or_build(1, 8); // hit -> MRU = [1, 2]
                              // Inserting a fresh key must evict seed=2 (LRU), not seed=1.
    cache.get_or_build(3, 8);
    assert_eq!(cache.len(), 2);
    // A subsequent request for seed=1 stays a hit (len stable at 2).
    let before = cache.len();
    let _ = cache.get_or_build(1, 8);
    assert_eq!(cache.len(), before);
}

#[test]
fn clear_empties_cache() {
    let cache = RotationCache::new(4);
    cache.get_or_build(42, 8);
    cache.get_or_build(43, 8);
    assert_eq!(cache.len(), 2);
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn zero_capacity_still_returns_valid_matrices() {
    let cache = RotationCache::new(0);
    let m = cache.get_or_build(42, 8);
    assert_eq!(m.dimension(), 8);
    assert_eq!(cache.len(), 0);
}

#[test]
fn different_dimensions_are_distinct_keys() {
    let cache = RotationCache::new(4);
    let a = cache.get_or_build(42, 8);
    let b = cache.get_or_build(42, 16);
    assert_eq!(a.dimension(), 8);
    assert_eq!(b.dimension(), 16);
    assert_eq!(cache.len(), 2);
}
