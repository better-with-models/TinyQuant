//! Integration tests for [`EntryMetaValue`].
//!
//! Key contracts tested here:
//! - `Float(NaN) == Float(NaN)` for Python dict-key stability.
//! - Clone is cheap (Arc bump, not deep copy).
//! - All variants implement `Eq`.

use std::sync::Arc;

use tinyquant_core::corpus::EntryMetaValue;

// ── Float NaN equality ────────────────────────────────────────────────────────

#[test]
fn entry_meta_value_nan_equal_for_dict_stability() {
    let a = EntryMetaValue::Float(f64::NAN);
    let b = EntryMetaValue::Float(f64::NAN);
    // IEEE 754 says NaN != NaN, but our bit-exact impl says they are equal.
    assert_eq!(a, b, "NaN must equal NaN for Python dict-key stability");
}

#[test]
fn entry_meta_value_nan_not_equal_to_non_nan() {
    let nan = EntryMetaValue::Float(f64::NAN);
    let one = EntryMetaValue::Float(1.0);
    assert_ne!(nan, one);
}

#[test]
fn entry_meta_value_float_bit_exact_distinguishes_pos_neg_zero() {
    // +0.0 and -0.0 have different bit patterns — they should compare unequal.
    let pos = EntryMetaValue::Float(0.0_f64);
    let neg = EntryMetaValue::Float(-0.0_f64);
    assert_ne!(pos, neg, "+0.0 and -0.0 have different bit patterns");
}

// ── Primitive variants ─────────────────────────────────────────────────────────

#[test]
fn null_equals_null() {
    assert_eq!(EntryMetaValue::Null, EntryMetaValue::Null);
}

#[test]
fn bool_variants_compare_correctly() {
    assert_eq!(EntryMetaValue::Bool(true), EntryMetaValue::Bool(true));
    assert_ne!(EntryMetaValue::Bool(true), EntryMetaValue::Bool(false));
}

#[test]
fn int_variants_compare_correctly() {
    assert_eq!(EntryMetaValue::Int(42), EntryMetaValue::Int(42));
    assert_ne!(EntryMetaValue::Int(1), EntryMetaValue::Int(2));
}

#[test]
fn string_variants_compare_correctly() {
    let a = EntryMetaValue::string("hello");
    let b = EntryMetaValue::string("hello");
    let c = EntryMetaValue::string("world");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn bytes_variants_compare_correctly() {
    let a = EntryMetaValue::bytes(b"data");
    let b = EntryMetaValue::bytes(b"data");
    let c = EntryMetaValue::bytes(b"other");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// ── Cross-variant inequality ───────────────────────────────────────────────────

#[test]
fn different_variants_are_not_equal() {
    assert_ne!(EntryMetaValue::Null, EntryMetaValue::Bool(false));
    assert_ne!(EntryMetaValue::Int(0), EntryMetaValue::Float(0.0));
    assert_ne!(EntryMetaValue::Bool(true), EntryMetaValue::Int(1));
}

// ── Clone cost ────────────────────────────────────────────────────────────────

#[test]
fn clone_string_is_arc_bump_not_deep_copy() {
    let big = "x".repeat(100_000);
    let val = EntryMetaValue::String(Arc::from(big.as_str()));
    // Clone should be O(1) — just a reference-count bump.
    let start = std::time::Instant::now();
    for _ in 0..10_000 {
        let _ = val.clone();
    }
    let elapsed = start.elapsed();
    // 10k clones of a large Arc should finish in well under 10 ms.
    assert!(
        elapsed.as_millis() < 10,
        "Clone took too long ({elapsed:?}), expected O(1) Arc bump"
    );
}

#[test]
fn clone_array_is_arc_bump_not_deep_copy() {
    let big_arr: Vec<EntryMetaValue> = (0..10_000).map(EntryMetaValue::Int).collect();
    let val = EntryMetaValue::array(big_arr);

    let start = std::time::Instant::now();
    for _ in 0..10_000 {
        let _ = val.clone();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 10,
        "Array clone took too long ({elapsed:?}), expected O(1) Arc bump"
    );
}

// ── Convenience constructors ───────────────────────────────────────────────────

#[test]
fn from_impls_work() {
    assert_eq!(EntryMetaValue::from(true), EntryMetaValue::Bool(true));
    assert_eq!(EntryMetaValue::from(42_i64), EntryMetaValue::Int(42));
    assert_eq!(EntryMetaValue::from(7_i32), EntryMetaValue::Int(7));
    let f: EntryMetaValue = 1.234_f64.into();
    assert!(matches!(f, EntryMetaValue::Float(_)));
    let s: EntryMetaValue = "hi".into();
    assert!(matches!(s, EntryMetaValue::String(_)));
}

// ── Accessor helpers ──────────────────────────────────────────────────────────

#[test]
fn accessor_helpers_return_correct_values() {
    assert!(EntryMetaValue::Null.is_null());
    assert_eq!(EntryMetaValue::Bool(false).as_bool(), Some(false));
    assert_eq!(EntryMetaValue::Int(-1).as_int(), Some(-1));
    assert!((EntryMetaValue::Float(1.5).as_float().unwrap() - 1.5).abs() < f64::EPSILON);
    assert_eq!(EntryMetaValue::string("abc").as_str(), Some("abc"));
    assert_eq!(
        EntryMetaValue::bytes(b"xy").as_bytes(),
        Some(b"xy".as_ref())
    );
}
