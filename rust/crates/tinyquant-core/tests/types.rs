//! Integration tests for shared type aliases.
//!
//! These verify the alias definitions are usable, have the right
//! primitive properties, and that `Arc<str>` clones are cheap
//! (no heap copy — pointer bump only).

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec;

use tinyquant_core::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};

#[test]
fn vector_id_clone_shares_allocation() {
    let id: VectorId = Arc::from("vec-001");
    let clone = id.clone();
    // Both point to the same heap allocation.
    assert!(Arc::ptr_eq(&id, &clone));
    assert_eq!(id.as_ref(), clone.as_ref());
}

#[test]
fn config_hash_roundtrips_as_str() {
    let h: ConfigHash = Arc::from("deadbeefcafe0123");
    assert_eq!(h.as_ref(), "deadbeefcafe0123");
}

#[test]
fn corpus_id_is_arc_str() {
    let c: CorpusId = Arc::from("corpus-test-1");
    let c2 = c.clone();
    assert!(Arc::ptr_eq(&c, &c2));
}

#[test]
fn vector_owns_f32_buffer() {
    let v: Vector = vec![1.0_f32, 2.0, 3.0];
    assert_eq!(v.len(), 3);
    assert_eq!(v[1], 2.0_f32);
}

#[test]
fn vector_slice_borrows_without_copy() {
    let owned: Vector = vec![0.0_f32; 16];
    let slice: VectorSlice<'_> = &owned;
    assert_eq!(slice.len(), 16);
    // Same pointer — no copy
    assert_eq!(slice.as_ptr(), owned.as_ptr());
}

#[test]
fn timestamp_is_i64() {
    let ts: Timestamp = -1_i64;
    assert_eq!(ts, -1_i64);
}
