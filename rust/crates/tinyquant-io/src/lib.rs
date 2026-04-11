//! Serialization, mmap, and file I/O for `TinyQuant`.
//!
//! This crate provides:
//!
//! - [`compressed_vector::to_bytes`] — encode a [`tinyquant_core::codec::CompressedVector`]
//!   to the Level-1 binary wire format.
//! - [`compressed_vector::from_bytes`] — decode from the wire format.
//! - [`errors::IoError`] — error type for all I/O operations.
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

pub mod compressed_vector;
pub mod errors;
