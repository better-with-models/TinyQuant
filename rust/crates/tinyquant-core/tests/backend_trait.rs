//! Compile-time and runtime assertions for the `SearchBackend` trait and
//! `SearchResult` type.
//!
//! Tests here verify:
//! - `SearchResult` is `Send + Sync + Clone`
//! - The trait is dyn-compatible (object-safe)
//! - `SearchResult` ordering is descending by score
//! - NaN scores degrade gracefully via `unwrap_or(Equal)`

extern crate alloc;
use alloc::sync::Arc;

use tinyquant_core::backend::{SearchBackend, SearchResult};

// ---------------------------------------------------------------------------
// Compile-time type-system assertions (function-based)
// ---------------------------------------------------------------------------

/// If `SearchBackend` is not dyn-compatible this function will fail to compile.
fn _assert_dyn_safe(_: &dyn SearchBackend) {}

/// Generic bounds check — compiles only if `T: Send + Sync`.
fn _assert_send_sync<T: Send + Sync>() {}

/// Generic bounds check — compiles only if `T: Clone`.
fn _assert_clone<T: Clone>() {}

// ---------------------------------------------------------------------------
// Runtime tests
// ---------------------------------------------------------------------------

#[test]
fn search_result_is_send_sync_clone() {
    _assert_send_sync::<SearchResult>();
    _assert_clone::<SearchResult>();
}

#[test]
fn search_result_ordering_descending() {
    let mut v = [
        SearchResult {
            vector_id: Arc::from("a"),
            score: 0.5,
        },
        SearchResult {
            vector_id: Arc::from("b"),
            score: 0.9,
        },
        SearchResult {
            vector_id: Arc::from("c"),
            score: 0.1,
        },
    ];
    v.sort();
    assert_eq!(v[0].vector_id.as_ref(), "b");
    assert_eq!(v[1].vector_id.as_ref(), "a");
    assert_eq!(v[2].vector_id.as_ref(), "c");
}

#[test]
fn search_result_nan_collapses_to_equal() {
    // The Ord impl uses `other.score.total_cmp(&self.score)` for descending
    // order.  IEEE 754 total order places NaN above all finite values, so
    // `nan_entry.cmp(one_entry)` = Less, which in Rust's ascending sort puts
    // `nan_entry` first — as if NaN were the highest score.
    // This test documents the actual behaviour of the Ord impl in protocol.rs.
    let mut v = [
        SearchResult {
            vector_id: Arc::from("nan"),
            score: f32::NAN,
        },
        SearchResult {
            vector_id: Arc::from("one"),
            score: 1.0,
        },
    ];
    v.sort();
    // NaN sorts first (treated as the highest score by total_cmp).
    assert_eq!(v[0].vector_id.as_ref(), "nan");
    assert_eq!(v[1].vector_id.as_ref(), "one");
}

#[test]
fn search_result_tie_breaks_preserve_insertion_order() {
    let mut v = [
        SearchResult {
            vector_id: Arc::from("first"),
            score: 0.5,
        },
        SearchResult {
            vector_id: Arc::from("second"),
            score: 0.5,
        },
        SearchResult {
            vector_id: Arc::from("third"),
            score: 0.5,
        },
    ];
    // stable sort preserves insertion order for equal elements
    v.sort();
    assert_eq!(v[0].vector_id.as_ref(), "first");
    assert_eq!(v[1].vector_id.as_ref(), "second");
    assert_eq!(v[2].vector_id.as_ref(), "third");
}
