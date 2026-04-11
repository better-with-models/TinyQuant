//! Decode a `CompressedVector` from its binary wire format.

use crate::compressed_vector::header::{decode_header, FORMAT_VERSION, HEADER_SIZE};
use crate::compressed_vector::unpack::unpack_indices;
use crate::errors::IoError;
use tinyquant_core::codec::CompressedVector;
use tinyquant_core::errors::CodecError;

const SUPPORTED_BIT_WIDTHS: &[u8] = &[2, 4, 8];

/// Deserialize a `CompressedVector` from `data`.
///
/// # Errors
///
/// Returns [`IoError`] if the data is malformed, truncated, has an unknown
/// version, or contains an invalid bit-width or residual flag.
pub fn from_bytes(data: &[u8]) -> Result<CompressedVector, IoError> {
    let (version, config_hash, dimension, bit_width) = decode_header(data)?;

    if version != FORMAT_VERSION {
        return Err(IoError::UnknownVersion { got: version });
    }
    if !SUPPORTED_BIT_WIDTHS.contains(&bit_width) {
        return Err(IoError::InvalidBitWidth { got: bit_width });
    }

    let dim = dimension as usize;
    let packed_len = (dim * bit_width as usize + 7) / 8;
    let payload_start = HEADER_SIZE;
    let flag_offset = payload_start + packed_len;

    if data.len() < flag_offset + 1 {
        return Err(IoError::Truncated {
            needed: flag_offset + 1,
            got: data.len(),
        });
    }

    // Unpack indices
    let packed = data
        .get(payload_start..flag_offset)
        .ok_or(IoError::Truncated {
            needed: flag_offset,
            got: data.len(),
        })?;
    let mut indices = vec![0u8; dim];
    unpack_indices(packed, dim, bit_width, &mut indices);

    // Residual flag
    let residual_flag = data.get(flag_offset).copied().ok_or(IoError::Truncated {
        needed: flag_offset + 1,
        got: data.len(),
    })?;

    let residual: Option<Box<[u8]>> = match residual_flag {
        0x00 => None,
        0x01 => {
            let rlen_start = flag_offset + 1;
            let rlen_end = rlen_start + 4;
            if data.len() < rlen_end {
                return Err(IoError::Truncated {
                    needed: rlen_end,
                    got: data.len(),
                });
            }
            let rlen_bytes: [u8; 4] = data
                .get(rlen_start..rlen_end)
                .ok_or(IoError::Truncated {
                    needed: rlen_end,
                    got: data.len(),
                })?
                .try_into()
                .map_err(|_| IoError::Truncated {
                    needed: rlen_end,
                    got: data.len(),
                })?;
            let rlen = u32::from_le_bytes(rlen_bytes) as usize;
            let rdata_start = rlen_end;
            let rdata_end = rdata_start + rlen;
            if data.len() < rdata_end {
                return Err(IoError::Truncated {
                    needed: rdata_end,
                    got: data.len(),
                });
            }
            let rdata = data.get(rdata_start..rdata_end).ok_or(IoError::Truncated {
                needed: rdata_end,
                got: data.len(),
            })?;
            Some(rdata.to_vec().into_boxed_slice())
        }
        got => {
            return Err(IoError::Decode(CodecError::InvalidResidualFlag { got }));
        }
    };

    let cv = CompressedVector::new(
        indices.into_boxed_slice(),
        residual,
        std::sync::Arc::from(config_hash),
        dimension,
        bit_width,
    )?;
    Ok(cv)
}
