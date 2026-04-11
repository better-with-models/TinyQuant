//! I/O and serialization errors for `tinyquant-io`.

use thiserror::Error;

/// Errors produced by serialization and file I/O operations.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum IoError {
    /// The input slice is shorter than expected.
    #[error("data too short: needed {needed} bytes, got {got}")]
    Truncated {
        /// Minimum bytes required.
        needed: usize,
        /// Bytes actually available.
        got: usize,
    },

    /// The format version byte is not recognized.
    #[error("unknown format version {got:#04x}")]
    UnknownVersion {
        /// The version byte found in the data.
        got: u8,
    },

    /// The `bit_width` field in the serialized data is not in `{2, 4, 8}`.
    #[error("invalid bit_width {got} in serialized data")]
    InvalidBitWidth {
        /// The invalid bit-width value.
        got: u8,
    },

    /// The `config_hash` field contains bytes that are not valid UTF-8.
    #[error("invalid UTF-8 in config hash")]
    InvalidUtf8,

    /// A length in the serialized data does not match the expected value.
    #[error("input/output length mismatch")]
    LengthMismatch,

    /// An inner codec operation failed during decode.
    #[error("decode error: {0}")]
    Decode(#[from] tinyquant_core::errors::CodecError),

    /// A file system I/O error (used in Phase 17+ file-path APIs).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
