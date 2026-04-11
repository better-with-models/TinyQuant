---
title: "Phase 12: Shared Types and Error Enums"
tags:
  - plans
  - rust
  - phase-12
  - types
  - errors
date-created: 2026-04-10
status: complete
category: planning
---

# Phase 12: Shared Types and Error Enums

> [!info] Goal
> Populate `tinyquant-core::types` and `tinyquant-core::errors` with
> the shared type aliases and `thiserror` enums that every subsequent
> phase will consume, and wire up `tinyquant-core::prelude` to
> re-export them.

> [!note] Reference docs
> - [[design/rust/type-mapping|Type Mapping]]
> - [[design/rust/error-model|Error Model]]

## Prerequisites

- Phase 11 complete (workspace scaffold green).

## Deliverables

### Files to modify / create

| File | Purpose |
|------|---------|
| `rust/crates/tinyquant-core/src/types.rs` | Type aliases |
| `rust/crates/tinyquant-core/src/errors.rs` | Error enums |
| `rust/crates/tinyquant-core/src/lib.rs` | `pub mod errors;` |
| `rust/crates/tinyquant-core/src/prelude.rs` | Re-export types & errors |
| `rust/crates/tinyquant-core/tests/types.rs` | Unit tests for aliases |
| `rust/crates/tinyquant-core/tests/errors.rs` | Unit tests for error display/source |

## Steps (TDD order)

- [ ] **Step 1: Write failing `tests/types.rs`**

```rust
//! Tests for shared type aliases.

use alloc::string::ToString;
extern crate alloc;

use tinyquant_core::types::{ConfigHash, CorpusId, Vector, VectorId, VectorSlice};

#[test]
fn vector_id_clone_is_cheap() {
    let id: VectorId = VectorId::from("abc");
    let clone = id.clone();
    assert_eq!(id.as_ref(), clone.as_ref());
}

#[test]
fn vector_type_owns_buffer() {
    let v: Vector = alloc::vec![1.0f32, 2.0, 3.0];
    assert_eq!(v.len(), 3);
}

#[test]
fn vector_slice_borrows() {
    let owned: Vector = alloc::vec![0.0f32; 8];
    let slice: VectorSlice<'_> = &owned;
    assert_eq!(slice.len(), 8);
}

#[test]
fn config_hash_and_corpus_id_are_cheap_clones() {
    let h: ConfigHash = ConfigHash::from("deadbeef");
    let c: CorpusId = CorpusId::from("corpus-1");
    assert_eq!(h.as_ref(), "deadbeef");
    assert_eq!(c.as_ref(), "corpus-1");
}
```

- [ ] **Step 2: Run — expect failure**

```bash
cd rust
cargo test -p tinyquant-core --test types
```

Expected: `types` module does not yet expose these names → compile
error.

- [ ] **Step 3: Implement `types.rs`**

```rust
//! Shared type aliases mirroring `tinyquant_cpu._types`.
//!
//! See `docs/design/rust/type-mapping.md` for the full mapping.

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// Stable identity for a vector inside a corpus.
///
/// `Arc<str>` matches Python's shareable `str` behavior with O(1)
/// clones.
pub type VectorId = Arc<str>;

/// SHA-256 hex digest identifying a `CodecConfig`.
pub type ConfigHash = Arc<str>;

/// Stable identity for a corpus.
pub type CorpusId = Arc<str>;

/// Owned 1-D FP32 vector.
pub type Vector = Vec<f32>;

/// Borrowed 1-D FP32 vector (immutable view).
pub type VectorSlice<'a> = &'a [f32];

/// Opaque nanoseconds-since-epoch counter, kept as `i64` to stay
/// `no_std`. Conversion to `std::time::SystemTime` happens at the
/// `tinyquant-io` boundary.
pub type Timestamp = i64;
```

- [ ] **Step 4: Run — expect pass**

```bash
cargo test -p tinyquant-core --test types
```

Expected: `4 passed`.

- [ ] **Step 5: Write failing `tests/errors.rs`**

```rust
//! Tests for error enums.

extern crate alloc;
use alloc::string::ToString;
use alloc::sync::Arc;

use tinyquant_core::errors::{BackendError, CodecError, CorpusError};

#[test]
fn unsupported_bit_width_display() {
    let e = CodecError::UnsupportedBitWidth { got: 3 };
    assert_eq!(
        e.to_string(),
        "bit_width must be one of [2, 4, 8], got 3",
    );
}

#[test]
fn dimension_mismatch_display() {
    let e = CodecError::DimensionMismatch { expected: 768, got: 1024 };
    assert_eq!(
        e.to_string(),
        "vector length 1024 does not match config dimension 768",
    );
}

#[test]
fn config_mismatch_display() {
    let e = CodecError::ConfigMismatch {
        expected: Arc::from("aaa"),
        got: Arc::from("bbb"),
    };
    assert_eq!(
        e.to_string(),
        "compressed config_hash \"bbb\" does not match config hash \"aaa\"",
    );
}

#[test]
fn corpus_error_wraps_codec_error_via_from() {
    let ce = CodecError::InvalidDimension { got: 0 };
    let corpus: CorpusError = ce.into();
    assert!(matches!(corpus, CorpusError::Codec(_)));
}

#[test]
fn backend_error_display_for_invalid_top_k() {
    let e = BackendError::InvalidTopK;
    assert_eq!(e.to_string(), "top_k must be > 0");
}
```

- [ ] **Step 6: Run — expect failure**

```bash
cargo test -p tinyquant-core --test errors
```

- [ ] **Step 7: Implement `errors.rs`**

```rust
//! Error enums for the TinyQuant core.
//!
//! Mirrors the Python `ValueError` subclasses under
//! `tinyquant_cpu.codec._errors`. See
//! `docs/design/rust/error-model.md` for full taxonomy and mapping.

use alloc::sync::Arc;
use thiserror::Error;

use crate::types::VectorId;

/// Errors produced by the codec layer.
#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// `bit_width` is outside `{2, 4, 8}`.
    #[error("bit_width must be one of [2, 4, 8], got {got}")]
    UnsupportedBitWidth { got: u8 },

    /// `dimension` is zero.
    #[error("dimension must be > 0, got {got}")]
    InvalidDimension { got: u32 },

    /// Input vector length does not match `config.dimension`.
    #[error("vector length {got} does not match config dimension {expected}")]
    DimensionMismatch { expected: u32, got: u32 },

    /// Codebook was trained under a different bit width.
    #[error("codebook bit_width {got} does not match config bit_width {expected}")]
    CodebookIncompatible { expected: u8, got: u8 },

    /// Compressed vector carries a different config hash than the
    /// supplied config.
    #[error("compressed config_hash {got:?} does not match config hash {expected:?}")]
    ConfigMismatch { expected: Arc<str>, got: Arc<str> },

    /// Codebook entry count is wrong for its bit width.
    #[error("codebook must have {expected} entries for bit_width={bit_width}, got {got}")]
    CodebookEntryCount { expected: u32, got: u32, bit_width: u8 },

    /// Codebook entries are not sorted ascending.
    #[error("codebook entries must be sorted ascending")]
    CodebookNotSorted,

    /// Codebook contains duplicate entries.
    #[error("codebook must contain {expected} distinct values, got {got}")]
    CodebookDuplicate { expected: u32, got: u32 },

    /// Not enough distinct training values to populate the codebook.
    #[error("insufficient training data for {expected} distinct entries")]
    InsufficientTrainingData { expected: u32 },

    /// An index referenced a position outside the codebook.
    #[error("index {index} is out of range [0, {bound})")]
    IndexOutOfRange { index: u8, bound: u32 },

    /// An internal length invariant was violated.
    #[error("input and output lengths disagree: {left} vs {right}")]
    LengthMismatch { left: usize, right: usize },
}

/// Errors produced by the corpus layer.
#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CorpusError {
    /// Upstream codec error.
    #[error(transparent)]
    Codec(#[from] CodecError),

    /// A vector id already exists in the corpus.
    #[error("duplicate vector_id {id:?}")]
    DuplicateVectorId { id: VectorId },

    /// A lookup for an unknown vector id failed.
    #[error("unknown vector_id {id:?}")]
    UnknownVectorId { id: VectorId },

    /// Attempt to change a frozen compression policy.
    #[error("compression policy is immutable once set")]
    PolicyImmutable,
}

/// Errors produced by the backend layer.
#[non_exhaustive]
#[derive(Debug, Error, Clone)]
pub enum BackendError {
    /// Backend contains no vectors (informational; not always fatal).
    #[error("backend is empty")]
    Empty,

    /// `top_k` parameter is zero or would overflow.
    #[error("top_k must be > 0")]
    InvalidTopK,

    /// Adapter-specific failure. String owned to stay `no_std` friendly.
    #[error("adapter error: {0}")]
    Adapter(Arc<str>),
}
```

- [ ] **Step 8: Update `lib.rs`**

```rust
pub mod errors;
pub mod prelude;
pub mod types;
```

- [ ] **Step 9: Update `prelude.rs`**

```rust
//! Convenience re-exports.
//!
//! Populated incrementally as each phase lands. Phase 12 adds the
//! shared types and error enums.

pub use crate::errors::{BackendError, CodecError, CorpusError};
pub use crate::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};
```

- [ ] **Step 10: Run errors test — expect pass**

```bash
cargo test -p tinyquant-core --test errors
```

- [ ] **Step 11: Run full workspace check**

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

- [ ] **Step 12: Commit**

```bash
git add rust/crates/tinyquant-core
git commit -m "feat(tinyquant-core): add shared types and error enums"
```

## Acceptance criteria

- Both new test files green.
- `tinyquant-core::prelude::*` exposes `CodecError`, `CorpusError`,
  `BackendError`, and all shared type aliases.
- Clippy clean.
- `no_std` build still works:
  `cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf`.

## Out of scope

- Any codec logic.
- Codec config / codebook / rotation matrix (phase 13+).
- Error mapping to Python exceptions (phase 22).

## See also

- [[plans/rust/phase-11-rust-workspace-scaffold|Phase 11]]
- [[plans/rust/phase-13-rotation-numerics|Phase 13]]
- [[design/rust/type-mapping|Type Mapping]]
- [[design/rust/error-model|Error Model]]
- [[superpowers/plans/2026-04-10-phase-12-shared-types-and-errors|Phase 12 Execution Plan]] — agentic task checklist for this phase
