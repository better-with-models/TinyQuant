//! `CompressedVector` binary serialization.
//!
//! Implements the Level-1 wire format defined in
//! `docs/design/rust/serialization-format.md`.

mod from_bytes;
pub(crate) mod header;
pub(crate) mod pack;
mod to_bytes;
pub(crate) mod unpack;

pub use from_bytes::from_bytes;
pub use header::HEADER_SIZE;
pub use to_bytes::to_bytes;
