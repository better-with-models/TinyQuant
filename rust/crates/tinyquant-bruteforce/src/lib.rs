//! Brute-force search backend for `TinyQuant`.
//!
//! Implements [`SearchBackend`] by computing cosine similarity against every
//! stored vector on each query.  Suitable for corpora up to ~100 k vectors.
//!
//! # Quick start
//!
//! ```rust
//! use tinyquant_bruteforce::BruteForceBackend;
//! use tinyquant_core::backend::SearchBackend;
//! use std::sync::Arc;
//!
//! let mut backend = BruteForceBackend::new();
//! let id: Arc<str> = Arc::from("vec-001");
//! backend.ingest(&[(id, vec![1.0, 0.0, 0.0])]).unwrap();
//! let results = backend.search(&[1.0, 0.0, 0.0], 1).unwrap();
//! assert_eq!(results.len(), 1);
//! ```
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
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]

pub(crate) mod backend;
pub(crate) mod errors;
pub(crate) mod similarity;
#[cfg(feature = "simd")]
pub(crate) mod similarity_simd;
pub(crate) mod store;

pub use backend::BruteForceBackend;
pub use errors::BackendError;
pub use tinyquant_core::backend::{SearchBackend, SearchResult};
