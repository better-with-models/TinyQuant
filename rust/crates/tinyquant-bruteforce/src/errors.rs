//! Error type re-export for `tinyquant-bruteforce`.
//!
//! This crate does not define new error variants; it uses `BackendError`
//! from `tinyquant-core` directly.  This module centralises the re-export
//! so that `lib.rs` and `backend.rs` share a single import point.

pub use tinyquant_core::errors::BackendError;
