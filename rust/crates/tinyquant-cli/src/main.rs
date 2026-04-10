//! `tinyquant` command-line tool.
//!
//! Standalone binary for codec operations, corpus management, and search.
//! Populated in Phase 22.
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

fn main() {
    println!("tinyquant {}", env!("CARGO_PKG_VERSION"));
}
