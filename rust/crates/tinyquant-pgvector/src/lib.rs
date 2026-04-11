//! pgvector-backed search backend for `TinyQuant`.
//!
//! Provides [`PgvectorAdapter`], an implementation of [`SearchBackend`] that
//! stores and queries vectors in a `PostgreSQL` database with the `pgvector`
//! extension installed.
//!
//! # Integration tests
//!
//! Tests that require a live `PostgreSQL` server are gated behind the
//! `test-containers` feature flag.  They are not run by the default
//! `cargo test` invocation.
#![deny(
    warnings,
    missing_docs,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cognitive_complexity
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    // `return` in `#[cfg(not(feature = "live-db"))]` blocks is "needless"
    // when building without the feature (since the cfg block IS the last
    // expression). Suppress globally for this crate; the pattern is
    // intentional and readable.
    clippy::needless_return
)]

pub(crate) mod adapter;
pub(crate) mod errors;
pub(crate) mod sql;
pub(crate) mod wire;

pub use adapter::PgvectorAdapter;
pub use errors::BackendError;
pub use tinyquant_core::backend::{SearchBackend, SearchResult};
