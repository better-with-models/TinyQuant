//! Search-backend trait and associated types.
//!
//! This module defines the uniform interface that every backend
//! implementation must satisfy.  Concrete implementations live in
//! `tinyquant-bruteforce` (Phase 19) and `tinyquant-pgvector` (Phase 19).
//!
//! # Quick start
//!
//! ```ignore
//! use tinyquant_core::backend::{SearchBackend, SearchResult};
//! ```

mod protocol;

pub use protocol::{SearchBackend, SearchResult};
