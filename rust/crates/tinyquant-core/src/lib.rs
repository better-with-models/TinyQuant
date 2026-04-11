//! CPU-only vector quantization codec — core types, codec, corpus, and
//! backend trait.
//!
//! This crate is `no_std` (requires `alloc` only). It contains no I/O,
//! no file-system access, and no platform-specific code.
#![no_std]
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

extern crate alloc;

pub mod codec;
pub mod errors;
pub mod prelude;
pub mod types;
