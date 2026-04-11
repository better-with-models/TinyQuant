//! Error helpers for `tinyquant-pgvector`.
//!
//! This crate does not define new error variants.  All errors are wrapped
//! into `BackendError::Adapter` so the caller sees a uniform error type.

use std::sync::Arc;

pub use tinyquant_core::errors::BackendError;

/// Wrap a formatted string into `BackendError::Adapter`.
pub fn adapter_err(msg: impl std::fmt::Display) -> BackendError {
    BackendError::Adapter(Arc::from(format!("{msg}")))
}

/// Wrap a `postgres::Error` into `BackendError::Adapter`.
///
/// Only available when the `live-db` feature is enabled.
#[cfg(feature = "live-db")]
pub fn from_pg(e: postgres::Error) -> BackendError {
    BackendError::Adapter(Arc::from(format!("pg: {e}")))
}
