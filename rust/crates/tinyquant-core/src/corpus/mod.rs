//! Corpus aggregate root and related domain types.
//!
//! # Module layout
//!
//! | Sub-module | Public items |
//! |---|---|
//! | `compression_policy` | [`CompressionPolicy`], [`StorageTag`] |
//! | `entry_meta_value` | [`EntryMetaValue`] |
//! | `vector_entry` | [`VectorEntry`] |
//! | `vector_id_map` | [`VectorIdMap`] (crate-private) |
//! | `events` | [`CorpusEvent`], [`ViolationKind`] |
//! | `errors` | [`CorpusError`] (re-export from `crate::errors`) |
//! | `corpus` | [`Corpus`], [`BatchReport`] |

mod aggregate;
pub mod compression_policy;
pub mod entry_meta_value;
pub mod errors;
pub mod events;
pub mod vector_entry;
pub(crate) mod vector_id_map;

pub use aggregate::{BatchReport, Corpus};
pub use compression_policy::{CompressionPolicy, StorageTag};
pub use entry_meta_value::EntryMetaValue;
pub use errors::CorpusError;
pub use events::{CorpusEvent, ViolationKind};
pub use vector_entry::VectorEntry;
