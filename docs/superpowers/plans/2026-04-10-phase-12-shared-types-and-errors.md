---
title: "Phase 12: Shared Types and Error Enums — Implementation Plan"
tags:
  - plans
  - rust
  - phase-12
  - superpowers
date-created: 2026-04-10
status: active
category: planning
---

# Phase 12: Shared Types & Error Enums — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate `tinyquant-core::types` and `tinyquant-core::errors` with all shared type aliases and `thiserror` error enums, wire them into the `prelude`, and verify `no_std` compliance — leaving every subsequent phase with a clean type foundation.

**Architecture:** `tinyquant-core` is a `no_std + alloc` leaf crate. Phase 11 scaffolded it with empty stubs for `types.rs` and `prelude.rs` and no `errors` module. Phase 12 fills those in. The key prerequisite is an MSRV bump from 1.78 → 1.81, which unlocks `core::error::Error` (stable in 1.81) and makes `thiserror v2` work in a `no_std` context — the error model design explicitly targets MSRV 1.81.

**Tech Stack:** Rust stable 1.81+, `thiserror v2` (no_std-capable), `alloc::sync::Arc`, `alloc::vec::Vec`.

---

## Critical prerequisite: MSRV gap

The current workspace pins `rust-version = "1.78"` and `thiserror = "1"`.

- `thiserror 1.x` uses `std::error::Error` — not available in `no_std`.
- `core::error::Error` is **stable only from Rust 1.81**.
- The error-model design doc (`docs/design/rust/error-model.md`) explicitly states: *"The design assumes `core::error::Error` is available (MSRV 1.81)."*

**Resolution:** bump MSRV to 1.81, update `rust-toolchain.toml` to channel `1.81`, and upgrade `thiserror` to `"2"` (which uses `core::error::Error` natively when no default features).

### Deviation from `type-mapping.md` for `CorpusError`

`docs/design/rust/type-mapping.md` shows `CorpusError` with two `#[from] CodecError` variants — a latent bug (a single enum cannot have two `#[from]` paths for the same source type; it is a compile error). The authoritative spec is `docs/design/rust/error-model.md`, which shows a manual `source()` that returns `Some(inner)` for the `Codec` variant. This plan reconciles both docs:

```rust
#[error("codec error: {0}")]
Codec(#[from] #[source] CodecError),
```

`#[from]` generates `impl From<CodecError> for CorpusError`; `#[source]` instructs thiserror v2 to make `source()` return `Some(self.0)`, matching the manual implementation shown in the error-model doc. Do not use `#[error(transparent)]` — it would delegate `source()` to the *inner error's own source* (which is `None` for leaf variants), losing the `source()` chain that the error-model design requires.

---

## File map

| File | Action | Responsibility |
|------|--------|---------------|
| `rust/rust-toolchain.toml` | Modify | Bump channel to `1.81` |
| `rust/Cargo.toml` | Modify | Bump `rust-version` to `"1.81"`, upgrade `thiserror` to `{ version = "2", default-features = false }` |
| `rust/crates/tinyquant-core/Cargo.toml` | Modify | Add `thiserror` dep, add `errors` note |
| `rust/crates/tinyquant-core/src/types.rs` | Replace stub | All type aliases (`VectorId`, `ConfigHash`, `CorpusId`, `Vector`, `VectorSlice`, `Timestamp`) |
| `rust/crates/tinyquant-core/src/errors.rs` | Create | `CodecError`, `CorpusError`, `BackendError` enums |
| `rust/crates/tinyquant-core/src/lib.rs` | Modify | Add `pub mod errors;` |
| `rust/crates/tinyquant-core/src/prelude.rs` | Replace stub | Re-export all types and errors |
| `rust/crates/tinyquant-core/tests/types.rs` | Create | Integration tests for type aliases |
| `rust/crates/tinyquant-core/tests/errors.rs` | Create | Integration tests for error display, From impls, and `core::error::Error::source` chaining |

---

## Task 1: Bump MSRV and upgrade thiserror

**Files:**
- Modify: `rust/rust-toolchain.toml`
- Modify: `rust/Cargo.toml`

- [ ] **Step 1: Open `rust/rust-toolchain.toml` and change the channel**

  Old:
  ```toml
  [toolchain]
  channel = "1.78.0"
  components = ["rustfmt", "clippy"]
  targets = ["thumbv7em-none-eabihf"]
  profile = "minimal"
  ```

  New:
  ```toml
  [toolchain]
  channel = "1.81.0"
  components = ["rustfmt", "clippy"]
  targets = ["thumbv7em-none-eabihf"]
  profile = "minimal"
  ```

- [ ] **Step 2: Update `rust/Cargo.toml` — bump `rust-version` and upgrade `thiserror`**

  In `[workspace.package]`, change:
  ```toml
  rust-version = "1.78"
  ```
  to:
  ```toml
  rust-version = "1.81"
  ```

  In `[workspace.dependencies]`, change:
  ```toml
  thiserror = "1"
  ```
  to:
  ```toml
  thiserror = { version = "2", default-features = false }
  ```

  The `default-features = false` enables `no_std` mode in thiserror v2 (it uses `core::fmt::Display` and `core::error::Error` instead of their `std` counterparts). `std` crates like `tinyquant-io` can still use it — there is no downside.

- [ ] **Step 3: Verify workspace still builds**

  ```bash
  cd rust
  cargo build --workspace --all-features
  ```

  Expected: zero compile errors (Phase 11 scaffold has no code that depends on the old thiserror). If `rustup` prompts to install 1.81.0, allow it.

- [ ] **Step 4: Commit**

  ```bash
  git add rust/rust-toolchain.toml rust/Cargo.toml
  git commit -m "chore(rust): bump MSRV to 1.81, upgrade thiserror to v2 for no_std support"
  ```

---

## Task 2: Write the failing types integration test

**Files:**
- Create: `rust/crates/tinyquant-core/tests/types.rs`

- [ ] **Step 1: Create `tests/types.rs`**

  ```rust
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
  ```

- [ ] **Step 2: Run — expect compile failure (types not yet defined)**

  ```bash
  cd rust
  cargo test -p tinyquant-core --test types 2>&1
  ```

  Expected: compile error — `tinyquant_core::types` exports nothing yet.

---

## Task 3: Implement `types.rs`

**Files:**
- Modify: `rust/crates/tinyquant-core/src/types.rs`

- [ ] **Step 1: Replace the stub with the full implementation**

  ```rust
  //! Shared primitive type aliases mirroring `tinyquant_cpu._types`.
  //!
  //! See `docs/design/rust/type-mapping.md` for the complete mapping
  //! table and rationale.

  use alloc::sync::Arc;

  /// Stable string identity for a vector inside a [`Corpus`].
  ///
  /// `Arc<str>` gives Python-like shareability (O(1) clones) without
  /// allocating a new buffer on each clone. Compare to Python `str`.
  pub type VectorId = Arc<str>;

  /// SHA-256 hex digest that uniquely identifies a [`CodecConfig`].
  ///
  /// Produced by [`CodecConfig::config_hash`] and stored in every
  /// [`CompressedVector`] for config-mismatch detection.
  pub type ConfigHash = Arc<str>;

  /// Stable string identity for a [`Corpus`].
  pub type CorpusId = Arc<str>;

  /// Owned, heap-allocated 1-D FP32 vector.
  ///
  /// Functions that only need to read a vector accept [`VectorSlice`]
  /// instead to avoid forcing a clone.
  // Use the full path to avoid a dead `use alloc::vec::Vec` import that
  // would trip `#![deny(warnings)]` since the alias itself is not used
  // as a value in this file.
  pub type Vector = alloc::vec::Vec<f32>;

  /// Borrowed immutable view of a 1-D FP32 vector.
  ///
  /// All codec read-path functions accept this instead of `&Vector`
  /// to be compatible with any contiguous FP32 backing store.
  pub type VectorSlice<'a> = &'a [f32];

  /// Nanoseconds since the Unix epoch, stored as a signed 64-bit integer.
  ///
  /// Kept as `i64` to stay `no_std` (no `std::time`). Crates with `std`
  /// access (`tinyquant-io`, `tinyquant-py`) convert to and from
  /// `std::time::SystemTime` at the boundary.
  pub type Timestamp = i64;
  ```

- [ ] **Step 2: Run the types test — expect pass**

  ```bash
  cd rust
  cargo test -p tinyquant-core --test types -- --nocapture
  ```

  Expected output: `6 tests passed`.

- [ ] **Step 3: Commit**

  ```bash
  git add rust/crates/tinyquant-core/src/types.rs \
          rust/crates/tinyquant-core/tests/types.rs
  git commit -m "feat(tinyquant-core): implement shared type aliases (types.rs)"
  ```

---

## Task 4: Write the failing errors integration test

**Files:**
- Create: `rust/crates/tinyquant-core/tests/errors.rs`

- [ ] **Step 1: Create `tests/errors.rs`**

  ```rust
  //! Integration tests for `tinyquant-core` error enums.
  //!
  //! Verifies:
  //!   - `Display` output matches Python's error message strings exactly
  //!   - `From` conversions (`CodecError` → `CorpusError`) work
  //!   - `core::error::Error::source` chains are correct
  //!   - Variants carry the right field types

  extern crate alloc;
  use alloc::string::ToString;
  use alloc::sync::Arc;

  use tinyquant_core::errors::{BackendError, CodecError, CorpusError};

  // --- CodecError display ---

  #[test]
  fn unsupported_bit_width_message() {
      let e = CodecError::UnsupportedBitWidth { got: 3 };
      assert_eq!(e.to_string(), "bit_width must be one of [2, 4, 8], got 3");
  }

  #[test]
  fn invalid_dimension_message() {
      let e = CodecError::InvalidDimension { got: 0 };
      assert_eq!(e.to_string(), "dimension must be > 0, got 0");
  }

  #[test]
  fn dimension_mismatch_message() {
      let e = CodecError::DimensionMismatch { expected: 768, got: 1024 };
      assert_eq!(
          e.to_string(),
          "vector length 1024 does not match config dimension 768",
      );
  }

  #[test]
  fn codebook_incompatible_message() {
      let e = CodecError::CodebookIncompatible { expected: 4, got: 8 };
      assert_eq!(
          e.to_string(),
          "codebook bit_width 8 does not match config bit_width 4",
      );
  }

  #[test]
  fn config_mismatch_message() {
      let e = CodecError::ConfigMismatch {
          expected: Arc::from("aaa"),
          got: Arc::from("bbb"),
      };
      assert_eq!(
          e.to_string(),
          r#"compressed config_hash "bbb" does not match config hash "aaa""#,
      );
  }

  #[test]
  fn codebook_entry_count_message() {
      let e = CodecError::CodebookEntryCount { expected: 16, got: 8, bit_width: 4 };
      assert_eq!(
          e.to_string(),
          "codebook must have 16 entries for bit_width=4, got 8",
      );
  }

  #[test]
  fn codebook_not_sorted_message() {
      let e = CodecError::CodebookNotSorted;
      assert_eq!(e.to_string(), "codebook entries must be sorted ascending");
  }

  #[test]
  fn codebook_duplicate_message() {
      let e = CodecError::CodebookDuplicate { expected: 16, got: 14 };
      assert_eq!(
          e.to_string(),
          "codebook must contain 16 distinct values, got 14",
      );
  }

  #[test]
  fn insufficient_training_data_message() {
      let e = CodecError::InsufficientTrainingData { expected: 16 };
      assert_eq!(
          e.to_string(),
          "insufficient training data for 16 distinct entries",
      );
  }

  #[test]
  fn index_out_of_range_message() {
      let e = CodecError::IndexOutOfRange { index: 20, bound: 16 };
      assert_eq!(e.to_string(), "index 20 is out of range [0, 16)");
  }

  #[test]
  fn length_mismatch_message() {
      let e = CodecError::LengthMismatch { left: 8, right: 4 };
      assert_eq!(e.to_string(), "input and output lengths disagree: 8 vs 4");
  }

  // --- CorpusError ---

  #[test]
  fn corpus_codec_wraps_via_from() {
      let ce = CodecError::InvalidDimension { got: 0 };
      let corpus: CorpusError = ce.clone().into();
      assert!(matches!(corpus, CorpusError::Codec(CodecError::InvalidDimension { got: 0 })));
  }

  #[test]
  fn corpus_error_source_is_codec_error() {
      // `#[source]` on the Codec variant makes thiserror v2 implement
      // `source()` as `Some(self.0)` — the inner CodecError itself, NOT
      // the codec error's own source. This matches the manual impl shown
      // in docs/design/rust/error-model.md.
      //
      // Do NOT use `#[error(transparent)]` — that would delegate to
      // `self.0.source()` (None for leaf variants), losing the chain.
      use core::error::Error;
      let ce = CodecError::DimensionMismatch { expected: 768, got: 100 };
      let corpus: CorpusError = ce.into();
      let source = corpus.source();
      assert!(source.is_some(), "CorpusError::Codec must expose the inner CodecError via source()");
      assert_eq!(
          source.unwrap().to_string(),
          "vector length 100 does not match config dimension 768",
      );
  }

  #[test]
  fn duplicate_vector_id_message() {
      use tinyquant_core::types::VectorId;
      let id: VectorId = Arc::from("v-001");
      let e = CorpusError::DuplicateVectorId { id };
      assert_eq!(e.to_string(), r#"duplicate vector_id "v-001""#);
  }

  #[test]
  fn unknown_vector_id_message() {
      use tinyquant_core::types::VectorId;
      let id: VectorId = Arc::from("v-999");
      let e = CorpusError::UnknownVectorId { id };
      assert_eq!(e.to_string(), r#"unknown vector_id "v-999""#);
  }

  #[test]
  fn policy_immutable_message() {
      let e = CorpusError::PolicyImmutable;
      assert_eq!(e.to_string(), "compression policy is immutable once set");
  }

  // --- BackendError ---

  #[test]
  fn backend_empty_message() {
      let e = BackendError::Empty;
      assert_eq!(e.to_string(), "backend is empty");
  }

  #[test]
  fn backend_invalid_top_k_message() {
      let e = BackendError::InvalidTopK;
      assert_eq!(e.to_string(), "top_k must be > 0");
  }

  #[test]
  fn backend_adapter_message() {
      let e = BackendError::Adapter(Arc::from("connection refused"));
      assert_eq!(e.to_string(), "adapter error: connection refused");
  }

  // --- Clone and PartialEq ---

  #[test]
  fn codec_error_clone_and_eq() {
      let a = CodecError::UnsupportedBitWidth { got: 3 };
      let b = a.clone();
      assert_eq!(a, b);
  }

  #[test]
  fn corpus_error_clone_and_eq() {
      let a = CorpusError::PolicyImmutable;
      let b = a.clone();
      assert_eq!(a, b);
  }

  // --- non_exhaustive guard: new variants can be added without breaking match ---
  // (compile-time property; no runtime assertion needed)
  ```

- [ ] **Step 2: Run — expect compile failure (errors module not yet defined)**

  ```bash
  cd rust
  cargo test -p tinyquant-core --test errors 2>&1
  ```

  Expected: compile error — `tinyquant_core::errors` does not exist.

---

## Task 5: Add `thiserror` to `tinyquant-core/Cargo.toml`

**Files:**
- Modify: `rust/crates/tinyquant-core/Cargo.toml`

- [ ] **Step 1: Add thiserror as a dep**

  Replace the current empty `[dependencies]` section:

  ```toml
  # NOTE: [dependencies] intentionally empty in Phase 11.
  # thiserror 1.x requires std::error::Error which is not in core until Rust 1.81+.
  # half, bytemuck, sha2, hex, faer, rand added in Phase 12 when used.
  [dependencies]
  ```

  With:

  ```toml
  [dependencies]
  # Phase 12: error enums (no_std-capable via thiserror v2 + default-features = false)
  thiserror = { workspace = true }
  ```

  Note: the workspace dep is now `{ version = "2", default-features = false }` after Task 1, so `{ workspace = true }` inherits the right settings.

- [ ] **Step 2: Verify workspace resolves**

  ```bash
  cd rust
  cargo metadata --no-deps -q > /dev/null
  ```

  Expected: exits 0. No output.

---

## Task 6: Implement `errors.rs`

**Files:**
- Create: `rust/crates/tinyquant-core/src/errors.rs`

- [ ] **Step 1: Create the file**

  ```rust
  //! Error enums for the TinyQuant core.
  //!
  //! Three enums cover the three architectural layers:
  //!
  //! - [`CodecError`] — codec layer (config, codebook, quantize, compress)
  //! - [`CorpusError`] — corpus aggregate layer
  //! - [`BackendError`] — backend/search layer
  //!
  //! All variants are `Clone + PartialEq` so they can be threaded
  //! through `Arc` and matched in tests.
  //!
  //! See `docs/design/rust/error-model.md` for the full taxonomy,
  //! Python-exception mapping, and FFI conversion table.

  use alloc::sync::Arc;
  use thiserror::Error;

  use crate::types::VectorId;

  /// Errors produced by the codec layer.
  ///
  /// Maps 1:1 to `tinyquant_cpu.codec._errors` Python exceptions; see the
  /// mapping table in `docs/design/rust/error-model.md`.
  #[non_exhaustive]
  #[derive(Debug, Error, Clone, PartialEq, Eq)]
  pub enum CodecError {
      /// `bit_width` is outside the supported set `{2, 4, 8}`.
      #[error("bit_width must be one of [2, 4, 8], got {got}")]
      UnsupportedBitWidth { got: u8 },

      /// `dimension` is zero — every codec operation requires a non-zero dim.
      #[error("dimension must be > 0, got {got}")]
      InvalidDimension { got: u32 },

      /// Input vector length does not match the config's declared dimension.
      #[error("vector length {got} does not match config dimension {expected}")]
      DimensionMismatch { expected: u32, got: u32 },

      /// Codebook was trained under a different `bit_width` than the config.
      #[error("codebook bit_width {got} does not match config bit_width {expected}")]
      CodebookIncompatible { expected: u8, got: u8 },

      /// A `CompressedVector` carries a `config_hash` that differs from the
      /// supplied `CodecConfig`.
      #[error("compressed config_hash {got:?} does not match config hash {expected:?}")]
      ConfigMismatch {
          expected: Arc<str>,
          got: Arc<str>,
      },

      /// Codebook entry count is inconsistent with `bit_width`.
      #[error("codebook must have {expected} entries for bit_width={bit_width}, got {got}")]
      CodebookEntryCount {
          expected: u32,
          got: u32,
          bit_width: u8,
      },

      /// Codebook entries are not sorted in non-decreasing order.
      #[error("codebook entries must be sorted ascending")]
      CodebookNotSorted,

      /// Codebook contains non-unique entries (violates `np.unique` invariant).
      #[error("codebook must contain {expected} distinct values, got {got}")]
      CodebookDuplicate { expected: u32, got: u32 },

      /// Training data contains fewer distinct values than needed to fill
      /// the codebook.
      #[error("insufficient training data for {expected} distinct entries")]
      InsufficientTrainingData { expected: u32 },

      /// A codebook index exceeds the codebook's valid range `[0, 2^bit_width)`.
      #[error("index {index} is out of range [0, {bound})")]
      IndexOutOfRange { index: u8, bound: u32 },

      /// An internal length invariant was violated (caller or internal bug).
      #[error("input and output lengths disagree: {left} vs {right}")]
      LengthMismatch { left: usize, right: usize },
  }

  /// Errors produced by the corpus aggregate layer.
  #[non_exhaustive]
  #[derive(Debug, Error, Clone, PartialEq, Eq)]
  pub enum CorpusError {
      /// An upstream codec operation failed.
      ///
      /// `#[from]` generates `impl From<CodecError> for CorpusError`.
      /// `#[source]` makes thiserror implement `source()` as `Some(self.0)`,
      /// so the inner `CodecError` is accessible via the standard error chain —
      /// matching the manual `impl` shown in `docs/design/rust/error-model.md`.
      ///
      /// Do NOT use `#[error(transparent)]` — that delegates `source()` to
      /// `self.0.source()` (None for leaf variants), losing the chain link.
      #[error("codec error: {0}")]
      Codec(#[from] #[source] CodecError),

      /// A vector with this id already exists in the corpus.
      #[error("duplicate vector_id {id:?}")]
      DuplicateVectorId { id: VectorId },

      /// A lookup requested a `VectorId` that does not exist.
      #[error("unknown vector_id {id:?}")]
      UnknownVectorId { id: VectorId },

      /// An attempt was made to change the compression policy after it was
      /// frozen.
      #[error("compression policy is immutable once set")]
      PolicyImmutable,
  }

  /// Errors produced by the search-backend layer.
  #[non_exhaustive]
  #[derive(Debug, Error, Clone)]
  pub enum BackendError {
      /// The backend contains no vectors — search cannot proceed.
      ///
      /// This is informational; callers may treat it as an empty result
      /// rather than a hard failure.
      #[error("backend is empty")]
      Empty,

      /// `top_k` was zero or would overflow internal heap sizing.
      #[error("top_k must be > 0")]
      InvalidTopK,

      /// An adapter-specific (e.g. pgvector) failure, with a human-readable
      /// message. `Arc<str>` avoids allocation on the hot path when the
      /// error is not inspected.
      #[error("adapter error: {0}")]
      Adapter(Arc<str>),
  }
  ```

  > **Note on `PartialEq` for `BackendError`:** `BackendError::Adapter` wraps
  > `Arc<str>` which is `PartialEq`, so `BackendError` derives `Clone` only.
  > Adding `PartialEq` here is straightforward but not required for Phase 12;
  > tests use pattern matching. If downstream code needs `PartialEq` on
  > `BackendError`, add it then.

---

## Task 7: Wire `errors` into `lib.rs` and `prelude.rs`

**Files:**
- Modify: `rust/crates/tinyquant-core/src/lib.rs`
- Modify: `rust/crates/tinyquant-core/src/prelude.rs`

- [ ] **Step 1: Add `pub mod errors;` to `lib.rs`**

  Current `lib.rs` ends with:
  ```rust
  pub mod prelude;
  pub mod types;
  ```

  Change to:
  ```rust
  pub mod errors;
  pub mod prelude;
  pub mod types;
  ```

  Keep the full `#![deny(...)]` and `#![no_std]` header unchanged.

- [ ] **Step 2: Replace the `prelude.rs` stub**

  ```rust
  //! Convenience re-exports for glob import (`use tinyquant_core::prelude::*`).
  //!
  //! Populated incrementally as each phase lands. After Phase 12:
  //! shared type aliases and error enums are available. Codec types,
  //! corpus, and backend are re-exported from Phase 13 onward.

  pub use crate::errors::{BackendError, CodecError, CorpusError};
  pub use crate::types::{ConfigHash, CorpusId, Timestamp, Vector, VectorId, VectorSlice};
  ```

---

## Task 8: Run the errors tests — expect pass

- [ ] **Step 1: Run the errors integration test**

  ```bash
  cd rust
  cargo test -p tinyquant-core --test errors -- --nocapture
  ```

  Expected: `21 tests passed`.

- [ ] **Step 2: Run the types test again (regression check)**

  ```bash
  cargo test -p tinyquant-core --test types -- --nocapture
  ```

  Expected: `6 tests passed` (unchanged from Task 3).

- [ ] **Step 3: Run all tinyquant-core tests**

  ```bash
  cargo test -p tinyquant-core
  ```

  Expected: smoke + types + errors = `28 tests passed, 0 failed`.

- [ ] **Step 4: Commit**

  ```bash
  git add rust/crates/tinyquant-core/src/errors.rs \
          rust/crates/tinyquant-core/src/lib.rs \
          rust/crates/tinyquant-core/src/prelude.rs \
          rust/crates/tinyquant-core/Cargo.toml \
          rust/crates/tinyquant-core/tests/errors.rs
  git commit -m "feat(tinyquant-core): implement error enums (CodecError, CorpusError, BackendError)"
  ```

---

## Task 9: Lint, format, and `no_std` verification

- [ ] **Step 1: Run clippy across the workspace**

  ```bash
  cd rust
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```

  Expected: zero warnings. Possible first-pass failures and how to fix them:

  | Clippy lint | Likely cause | Fix |
  |---|---|---|
  | `clippy::doc_markdown` | Bare `CodecConfig` in a doc comment | Wrap in backticks: `` `CodecConfig` `` |
  | `clippy::module_name_repetitions` | Already `#[allow]`'d in lib.rs — no action |
  | `unused_import` | `use alloc::vec::Vec` in types.rs if Vector alias not used in the same file | Move to where it's needed, or suppress with a doc comment explaining it's re-exported |

- [ ] **Step 2: Run `cargo fmt` check**

  ```bash
  cargo fmt --all -- --check
  ```

  If it fails:
  ```bash
  cargo fmt --all
  git add -p   # review then stage
  ```

- [ ] **Step 3: Verify `no_std` build for embedded target**

  ```bash
  cargo build -p tinyquant-core --no-default-features \
      --target thumbv7em-none-eabihf
  ```

  Expected: compiles successfully. This target has no `std` or `alloc` by default; the `extern crate alloc;` in `lib.rs` provides the allocator surface. `thiserror v2 default-features=false` uses only `core::fmt` and `core::error::Error` (stable 1.81), both of which are available here.

  If you see: `error[E0433]: failed to resolve: use of undeclared crate or module 'std'`, it means something in your code or a dep pulled in `std`. Trace the error and remove the offending import.

- [ ] **Step 4: Confirm Python tests still pass (no regressions)**

  ```bash
  cd ..   # repo root
  ruff check . && mypy --strict src/ tests/
  pytest --tb=short -q
  ```

  Expected: `214 passed (or ≥ 208 passed, 6 skipped)` — same as the Phase 11 baseline.

- [ ] **Step 5: Final commit**

  ```bash
  cd rust
  git add .   # pick up any fmt changes
  git commit -m "chore(tinyquant-core): clippy clean, fmt, no_std build verified"
  ```

---

## Task 10: Update wiki docs

**Files:**
- Modify: `docs/plans/rust/phase-12-shared-types-and-errors.md` — mark `status: complete`
- Modify: `docs/roadmap.md` — Phase 12 row: `**active**` → `**complete**`
- Append: `docs/log.md` — log entry for Phase 12

- [ ] **Step 1: Mark the wiki plan complete**

  In `docs/plans/rust/phase-12-shared-types-and-errors.md` frontmatter, change:
  ```yaml
  status: draft
  ```
  to:
  ```yaml
  status: complete
  ```

- [ ] **Step 2: Update roadmap**

  In `docs/roadmap.md`, in the Rust phases table, find:
  ```
  | 12 | Shared Types & Errors | **active** | tinyquant-core (types, errors) | Phase 11 | ...
  ```
  Change `**active**` → `**complete**`.

  Also update the Gantt chart entry from `:active, p12,` to `:done, p12,`.

- [ ] **Step 3: Append to `docs/log.md`**

  ```markdown
  ## 2026-04-10 — Phase 12 complete

  Populated `tinyquant-core::types` (6 type aliases) and
  `tinyquant-core::errors` (`CodecError` with 11 variants,
  `CorpusError` with 4 variants, `BackendError` with 3 variants).
  Bumped MSRV 1.78 → 1.81 and thiserror 1 → 2 to enable `no_std`
  error derives via `core::error::Error`. All 28 `tinyquant-core`
  tests pass; `no_std` build on `thumbv7em-none-eabihf` verified.
  ```

- [ ] **Step 4: Commit docs**

  ```bash
  git add docs/plans/rust/phase-12-shared-types-and-errors.md \
          docs/roadmap.md \
          docs/log.md
  git commit -m "docs: mark Phase 12 complete in roadmap and log"
  ```

---

## Acceptance criteria

All of the following must be true before this phase is considered done:

- [ ] `cargo test -p tinyquant-core` passes with ≥ 28 tests (smoke + types + errors).
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0.
- [ ] `cargo fmt --all -- --check` exits 0.
- [ ] `cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf` exits 0.
- [ ] `pytest` passes at the Python-suite baseline (no regressions).
- [ ] `docs/roadmap.md` shows Phase 12 as `**complete**`.

---

## Out of scope

- Any codec logic (`CodecConfig`, `Codebook`, `RotationMatrix`, `CompressedVector`) — Phase 13+.
- SHA-256 hash computation — requires `sha2` dep, lands with `CodecConfig` in Phase 13.
- Error mapping to Python exceptions — Phase 22 (`tinyquant-py`).
- C ABI error codes (`TinyQuantErrorKind`) — Phase 22 (`tinyquant-sys`).
- `IoError` — Phase 16 (`tinyquant-io`).

---

## See also

- `docs/plans/rust/phase-11-rust-workspace-scaffold.md` — what Phase 11 delivered
- `docs/plans/rust/phase-13-rotation-numerics.md` — next phase
- `docs/design/rust/type-mapping.md` — authoritative type mapping
- `docs/design/rust/error-model.md` — error taxonomy and FFI/Python mappings
- `docs/design/rust/crate-topology.md` — file structure and layer rules
