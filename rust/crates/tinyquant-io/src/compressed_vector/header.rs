//! Header encoding/decoding for the `CompressedVector` binary format.
//!
//! Header layout (70 bytes, all fields little-endian):
//!
//! | offset | length | field       | encoding          |
//! |-------:|-------:|-------------|-------------------|
//! |      0 |      1 | version     | u8 = 0x01         |
//! |      1 |     64 | config_hash | UTF-8, NUL-padded |
//! |     65 |      4 | dimension   | u32 LE            |
//! |     69 |      1 | bit_width   | u8                |

use crate::errors::IoError;

pub const FORMAT_VERSION: u8 = 0x01;
pub const HASH_BYTES: usize = 64;
/// Total header size in bytes.
pub const HEADER_SIZE: usize = 70; // 1 + 64 + 4 + 1

/// Append a 70-byte header to `out`.
pub fn encode_header(out: &mut Vec<u8>, config_hash: &str, dimension: u32, bit_width: u8) {
    out.push(FORMAT_VERSION);
    let mut buf = [0u8; HASH_BYTES];
    let src = config_hash.as_bytes();
    let n = src.len().min(HASH_BYTES);
    #[allow(clippy::indexing_slicing)]
    // n <= HASH_BYTES by construction; buf is exactly HASH_BYTES
    buf[..n].copy_from_slice(&src[..n]);
    out.extend_from_slice(&buf);
    out.extend_from_slice(&dimension.to_le_bytes());
    out.push(bit_width);
}

/// Decode the header from the start of `data`.
///
/// Returns `(version, config_hash, dimension, bit_width)`.
pub fn decode_header(data: &[u8]) -> Result<(u8, &str, u32, u8), IoError> {
    if data.len() < HEADER_SIZE {
        return Err(IoError::Truncated {
            needed: HEADER_SIZE,
            got: data.len(),
        });
    }
    // Safety: bounds-checked above; index 0 is always valid
    #[allow(clippy::indexing_slicing)]
    let version = data[0];

    let hash_raw = data.get(1..65).ok_or(IoError::Truncated {
        needed: HEADER_SIZE,
        got: data.len(),
    })?;
    let trim = hash_raw.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let config_hash = core::str::from_utf8(hash_raw.get(..trim).unwrap_or(hash_raw))
        .map_err(|_| IoError::InvalidUtf8)?;

    let dim_bytes: [u8; 4] = data
        .get(65..69)
        .ok_or(IoError::Truncated {
            needed: HEADER_SIZE,
            got: data.len(),
        })?
        .try_into()
        .map_err(|_| IoError::Truncated {
            needed: HEADER_SIZE,
            got: data.len(),
        })?;
    let dimension = u32::from_le_bytes(dim_bytes);

    let bit_width = data.get(69).copied().ok_or(IoError::Truncated {
        needed: HEADER_SIZE,
        got: data.len(),
    })?;

    Ok((version, config_hash, dimension, bit_width))
}
