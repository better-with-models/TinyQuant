//! Thin re-export of [`CorpusError`] from [`crate::errors`].
//!
//! The canonical definition lives in [`crate::errors`] so that it shares one
//! `thiserror` derivation with [`CodecError`](crate::errors::CodecError) and
//! [`BackendError`](crate::errors::BackendError). This module re-exports the
//! type so that downstream code can write
//! `tinyquant_core::corpus::CorpusError` without reaching into the error
//! module directly.

pub use crate::errors::CorpusError;
