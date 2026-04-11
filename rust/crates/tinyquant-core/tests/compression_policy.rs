//! Integration tests for [`CompressionPolicy`] and [`StorageTag`].

use tinyquant_core::corpus::{CompressionPolicy, StorageTag};

#[test]
fn compress_policy_requires_codec() {
    assert!(CompressionPolicy::Compress.requires_codec());
    assert!(!CompressionPolicy::Passthrough.requires_codec());
    assert!(!CompressionPolicy::Fp16.requires_codec());
}

#[test]
fn storage_tag_mapping() {
    assert_eq!(CompressionPolicy::Compress.storage_tag(), StorageTag::U8);
    assert_eq!(
        CompressionPolicy::Passthrough.storage_tag(),
        StorageTag::F32
    );
    assert_eq!(CompressionPolicy::Fp16.storage_tag(), StorageTag::F16);
}

#[test]
fn compression_policy_clone_and_eq() {
    let p = CompressionPolicy::Compress;
    let q = p;
    assert_eq!(p, q);
    assert_ne!(CompressionPolicy::Compress, CompressionPolicy::Passthrough);
    assert_ne!(CompressionPolicy::Passthrough, CompressionPolicy::Fp16);
}

#[test]
fn storage_tag_clone_and_eq() {
    let t = StorageTag::F16;
    let u = t;
    assert_eq!(t, u);
    assert_ne!(StorageTag::U8, StorageTag::F32);
}

#[test]
fn violation_kind_python_tags_match_spec() {
    use tinyquant_core::corpus::ViolationKind;
    assert_eq!(
        ViolationKind::ConfigMismatch.as_python_tag(),
        "config_mismatch"
    );
    assert_eq!(
        ViolationKind::PolicyConflict.as_python_tag(),
        "policy_conflict"
    );
    assert_eq!(ViolationKind::DuplicateId.as_python_tag(), "duplicate_id");
    assert_eq!(
        ViolationKind::DimensionMismatch.as_python_tag(),
        "dimension_mismatch"
    );
}
