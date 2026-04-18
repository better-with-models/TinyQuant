//! Level-2 TQCV file header encode/decode.
//!
//! Fixed header layout (24 bytes):
//!
//! | offset | len | field           | notes                                       |
//! |--------|-----|-----------------|---------------------------------------------|
//! | 0      | 4   | magic           | `b"TQCV"` (final) or `b"TQCX"` (tentative) |
//! | 4      | 1   | format_version  | `0x01`                                      |
//! | 5      | 3   | reserved        | `[0x00; 3]`; reject if non-zero             |
//! | 8      | 8   | vector_count    | `u64` LE                                    |
//! | 16     | 4   | dimension       | `u32` LE, must be non-zero                  |
//! | 20     | 1   | bit_width       | one of `{2, 4, 8}`                          |
//! | 21     | 1   | residual_flag   | `0x00` = no residuals, `0x01` = residuals   |
//! | 22     | 2   | config_hash_len | `u16` LE, ≤ 256                             |

use crate::errors::IoError;

/// Magic bytes for a finalized corpus file.
pub const MAGIC_FINAL: &[u8; 4] = b"TQCV";
/// Magic bytes for a tentative (write-in-progress) corpus file.
pub const MAGIC_TENTATIVE: &[u8; 4] = b"TQCX";
/// Format version byte.
pub const FORMAT_VERSION: u8 = 0x01;
/// Size of the fixed 24-byte header.
pub const FIXED_HEADER_SIZE: usize = 24;
pub const MAX_CONFIG_HASH_LEN: usize = 256;
pub const MAX_METADATA_LEN: usize = 16 * 1024 * 1024; // 16 MiB

const SUPPORTED_BIT_WIDTHS: &[u8] = &[2, 4, 8];

/// Decoded Level-2 header with ownership of string data.
pub struct CorpusFileHeader {
    /// Number of vectors in the file.
    pub vector_count: u64,
    /// Dimension of each vector.
    pub dimension: u32,
    /// Bit width (2, 4, or 8).
    pub bit_width: u8,
    /// Whether all vectors have a residual section.
    pub residual: bool,
    /// Config hash string.
    pub config_hash: String,
    /// Opaque metadata bytes.
    pub metadata: Vec<u8>,
    /// Byte offset where the body (records) begins.
    pub body_offset: usize,
}

/// Encode the full Level-2 header (fixed + variable + padding).
///
/// Uses `TQCX` (tentative) magic; caller must flip to `TQCV` on finalize.
///
/// # Errors
///
/// Returns [`IoError::InvalidHeader`] if the config hash or metadata exceed
/// their maximum lengths, or if the dimension is zero.
/// Returns [`IoError::InvalidBitWidth`] if `bit_width ∉ {2, 4, 8}`.
pub fn encode_header(
    config_hash: &str,
    dimension: u32,
    bit_width: u8,
    residual: bool,
    metadata: &[u8],
    vector_count: u64,
) -> Result<Vec<u8>, IoError> {
    let hash_bytes = config_hash.as_bytes();
    if hash_bytes.len() > MAX_CONFIG_HASH_LEN {
        return Err(IoError::InvalidHeader);
    }
    if metadata.len() > MAX_METADATA_LEN {
        return Err(IoError::InvalidHeader);
    }
    if !SUPPORTED_BIT_WIDTHS.contains(&bit_width) {
        return Err(IoError::InvalidBitWidth { got: bit_width });
    }
    if dimension == 0 {
        return Err(IoError::InvalidHeader);
    }

    #[allow(clippy::cast_possible_truncation)]
    let config_hash_len = hash_bytes.len() as u16;
    #[allow(clippy::cast_possible_truncation)]
    let metadata_len = metadata.len() as u32;

    let capacity = FIXED_HEADER_SIZE + hash_bytes.len() + 4 + metadata.len() + 7;
    let mut out = Vec::with_capacity(capacity);

    // Fixed header
    out.extend_from_slice(MAGIC_TENTATIVE); // offset 0..4
    out.push(FORMAT_VERSION); // offset 4
    out.extend_from_slice(&[0u8; 3]); // offset 5..8 reserved
    out.extend_from_slice(&vector_count.to_le_bytes()); // offset 8..16
    out.extend_from_slice(&dimension.to_le_bytes()); // offset 16..20
    out.push(bit_width); // offset 20
    out.push(u8::from(residual)); // offset 21
    out.extend_from_slice(&config_hash_len.to_le_bytes()); // offset 22..24

    // Variable prefix
    out.extend_from_slice(hash_bytes);
    out.extend_from_slice(&metadata_len.to_le_bytes());
    out.extend_from_slice(metadata);

    // Alignment padding to 8-byte boundary
    let pad = (8 - (out.len() % 8)) % 8;
    out.resize(out.len() + pad, 0x00);
    Ok(out)
}

/// Decode the fixed portion of the Level-2 header (magic, version, reserved,
/// counts, `bit_width`, `residual`, `config_hash_len`).
fn decode_fixed_header(data: &[u8]) -> Result<(u64, u32, u8, bool, usize), IoError> {
    let magic: [u8; 4] = data
        .get(0..4)
        .ok_or(IoError::Truncated {
            needed: 4,
            got: data.len(),
        })?
        .try_into()
        .map_err(|_| IoError::Truncated {
            needed: 4,
            got: data.len(),
        })?;

    match &magic {
        m if m == MAGIC_FINAL => {}
        m if m == MAGIC_TENTATIVE => {
            // The file was not finalized (magic is still TQCX). Fold this
            // into Truncated with coherent field values: the file exists but
            // is incomplete — signal that one more byte "would help" so callers
            // see a meaningful "needed > got" relationship.
            return Err(IoError::Truncated {
                needed: data.len() + 1,
                got: data.len(),
            });
        }
        _ => return Err(IoError::BadMagic { got: magic }),
    }

    let version = data.get(4).copied().ok_or(IoError::Truncated {
        needed: 5,
        got: data.len(),
    })?;
    if version != FORMAT_VERSION {
        return Err(IoError::UnknownVersion { got: version });
    }

    let reserved = data.get(5..8).ok_or(IoError::Truncated {
        needed: 8,
        got: data.len(),
    })?;
    if reserved != [0u8, 0, 0] {
        return Err(IoError::InvalidHeader);
    }

    let vc_bytes: [u8; 8] = data
        .get(8..16)
        .ok_or(IoError::Truncated {
            needed: 16,
            got: data.len(),
        })?
        .try_into()
        .map_err(|_| IoError::Truncated {
            needed: 16,
            got: data.len(),
        })?;
    let vector_count = u64::from_le_bytes(vc_bytes);

    let dim_bytes: [u8; 4] = data
        .get(16..20)
        .ok_or(IoError::Truncated {
            needed: 20,
            got: data.len(),
        })?
        .try_into()
        .map_err(|_| IoError::Truncated {
            needed: 20,
            got: data.len(),
        })?;
    let dimension = u32::from_le_bytes(dim_bytes);
    if dimension == 0 {
        return Err(IoError::InvalidHeader);
    }

    let bit_width = data.get(20).copied().ok_or(IoError::Truncated {
        needed: 21,
        got: data.len(),
    })?;
    if !SUPPORTED_BIT_WIDTHS.contains(&bit_width) {
        return Err(IoError::InvalidBitWidth { got: bit_width });
    }

    let residual_flag = data.get(21).copied().ok_or(IoError::Truncated {
        needed: 22,
        got: data.len(),
    })?;
    let residual = match residual_flag {
        0x00 => false,
        0x01 => true,
        _ => return Err(IoError::InvalidHeader),
    };

    let chl_bytes: [u8; 2] = data
        .get(22..24)
        .ok_or(IoError::Truncated {
            needed: 24,
            got: data.len(),
        })?
        .try_into()
        .map_err(|_| IoError::Truncated {
            needed: 24,
            got: data.len(),
        })?;
    let config_hash_len = u16::from_le_bytes(chl_bytes) as usize;
    if config_hash_len > MAX_CONFIG_HASH_LEN {
        return Err(IoError::InvalidHeader);
    }

    Ok((
        vector_count,
        dimension,
        bit_width,
        residual,
        config_hash_len,
    ))
}

/// Decode the Level-2 header from `data`.
///
/// # Errors
///
/// Returns [`IoError`] if the magic is wrong, data is truncated, reserved
/// bytes are non-zero, or any field value is out of range.
pub fn decode_header(data: &[u8]) -> Result<CorpusFileHeader, IoError> {
    if data.len() < FIXED_HEADER_SIZE {
        return Err(IoError::Truncated {
            needed: FIXED_HEADER_SIZE,
            got: data.len(),
        });
    }

    let (vector_count, dimension, bit_width, residual, config_hash_len) =
        decode_fixed_header(data)?;

    let (config_hash, metadata, body_offset) =
        decode_variable_prefix(data, FIXED_HEADER_SIZE, config_hash_len)?;

    Ok(CorpusFileHeader {
        vector_count,
        dimension,
        bit_width,
        residual,
        config_hash,
        metadata,
        body_offset,
    })
}

/// Decode the variable-length prefix and compute the body offset.
fn decode_variable_prefix(
    data: &[u8],
    start: usize,
    config_hash_len: usize,
) -> Result<(String, Vec<u8>, usize), IoError> {
    let mut pos = start;

    let hash_end = pos + config_hash_len;
    if data.len() < hash_end {
        return Err(IoError::Truncated {
            needed: hash_end,
            got: data.len(),
        });
    }
    let hash_bytes = data.get(pos..hash_end).ok_or(IoError::Truncated {
        needed: hash_end,
        got: data.len(),
    })?;
    let config_hash = std::str::from_utf8(hash_bytes)
        .map_err(|_| IoError::InvalidUtf8)?
        .to_owned();
    pos = hash_end;

    let ml_end = pos + 4;
    if data.len() < ml_end {
        return Err(IoError::Truncated {
            needed: ml_end,
            got: data.len(),
        });
    }
    let ml_bytes: [u8; 4] = data
        .get(pos..ml_end)
        .ok_or(IoError::Truncated {
            needed: ml_end,
            got: data.len(),
        })?
        .try_into()
        .map_err(|_| IoError::Truncated {
            needed: ml_end,
            got: data.len(),
        })?;
    let metadata_len = u32::from_le_bytes(ml_bytes) as usize;
    pos = ml_end;

    let meta_end = pos + metadata_len;
    if data.len() < meta_end {
        return Err(IoError::Truncated {
            needed: meta_end,
            got: data.len(),
        });
    }
    let metadata = data
        .get(pos..meta_end)
        .ok_or(IoError::Truncated {
            needed: meta_end,
            got: data.len(),
        })?
        .to_vec();
    pos = meta_end;

    let body_offset = ((pos + 7) / 8) * 8;

    Ok((config_hash, metadata, body_offset))
}
