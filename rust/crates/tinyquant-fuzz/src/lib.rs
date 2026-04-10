//! Fuzz targets for `TinyQuant`.
//!
//! Uses `libfuzzer-sys` (wired in a later phase). Targets added as each
//! serialization surface is implemented.
#![deny(warnings, missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]
