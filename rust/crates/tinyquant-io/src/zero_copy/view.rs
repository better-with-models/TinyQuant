//! Zero-copy view of a Level-1 serialized `CompressedVector`.

use crate::compressed_vector::header::{decode_header, FORMAT_VERSION, HEADER_SIZE};
use crate::compressed_vector::unpack::unpack_indices;
use crate::errors::IoError;
use std::sync::Arc;
use tinyquant_core::codec::CompressedVector;

const SUPPORTED_BIT_WIDTHS: &[u8] = &[2, 4, 8];

/// A zero-copy view of a single Level-1 serialized [`CompressedVector`].
///
/// All fields borrow from the input byte slice. No heap allocation occurs
/// during [`parse`](CompressedVectorView::parse) or
/// [`unpack_into`](CompressedVectorView::unpack_into).
///
/// ```compile_fail
/// fn assert_not_serialize<T: serde::Serialize>() {}
/// assert_not_serialize::<tinyquant_io::CompressedVectorView<'_>>();
/// ```
pub struct CompressedVectorView<'a> {
    /// Format version byte from the Level-1 header.
    pub format_version: u8,
    /// Config hash string borrowed from the Level-1 header.
    pub config_hash: &'a str,
    /// Number of dimensions.
    pub dimension: u32,
    /// Bit width (2, 4, or 8).
    pub bit_width: u8,
    /// Packed index bytes (LSB-first).
    pub packed_indices: &'a [u8],
    /// Residual bytes (2 bytes per dimension), or `None`.
    pub residual: Option<&'a [u8]>,
}

impl<'a> CompressedVectorView<'a> {
    /// Parse a single Level-1 record from the start of `data`.
    ///
    /// Returns `(view, unconsumed_tail)`.
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if the data is truncated, has an unknown version,
    /// an invalid bit-width, or a bad residual flag.
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), IoError> {
        let (version, config_hash, dimension, bit_width) = decode_header(data)?;
        if version != FORMAT_VERSION {
            return Err(IoError::UnknownVersion { got: version });
        }
        if !SUPPORTED_BIT_WIDTHS.contains(&bit_width) {
            return Err(IoError::InvalidBitWidth { got: bit_width });
        }
        let dim = dimension as usize;
        let packed_len = (dim * bit_width as usize).div_ceil(8);
        let flag_offset = HEADER_SIZE + packed_len;
        if data.len() < flag_offset + 1 {
            return Err(IoError::Truncated {
                needed: flag_offset + 1,
                got: data.len(),
            });
        }
        let packed_indices = data
            .get(HEADER_SIZE..flag_offset)
            .ok_or(IoError::Truncated {
                needed: flag_offset,
                got: data.len(),
            })?;
        let residual_flag = data.get(flag_offset).copied().ok_or(IoError::Truncated {
            needed: flag_offset + 1,
            got: data.len(),
        })?;
        let (residual, record_end) = parse_residual(data, flag_offset, residual_flag)?;
        let tail = data.get(record_end..).ok_or(IoError::Truncated {
            needed: record_end,
            got: data.len(),
        })?;
        Ok((
            Self {
                format_version: version,
                config_hash,
                dimension,
                bit_width,
                packed_indices,
                residual,
            },
            tail,
        ))
    }

    /// Unpack indices into caller-provided buffer (no allocation).
    ///
    /// # Errors
    ///
    /// Returns [`IoError::LengthMismatch`] if `out.len() != self.dimension as usize`.
    pub fn unpack_into(&self, out: &mut [u8]) -> Result<(), IoError> {
        if out.len() != self.dimension as usize {
            return Err(IoError::LengthMismatch);
        }
        unpack_indices(
            self.packed_indices,
            self.dimension as usize,
            self.bit_width,
            out,
        );
        Ok(())
    }

    /// Allocating escape hatch — copies borrowed bytes into an owned [`CompressedVector`].
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if unpacking or construction fails.
    pub fn to_owned_cv(&self) -> Result<CompressedVector, IoError> {
        let mut indices = vec![0u8; self.dimension as usize];
        self.unpack_into(&mut indices)?;
        let residual = self.residual.map(|r| r.to_vec().into_boxed_slice());
        let cv = CompressedVector::new(
            indices.into_boxed_slice(),
            residual,
            Arc::from(self.config_hash),
            self.dimension,
            self.bit_width,
        )?;
        Ok(cv)
    }
}

/// Parse the residual section starting at `flag_offset` in `data`.
/// Returns `(residual_slice_or_none, record_end)`.
fn parse_residual(
    data: &[u8],
    flag_offset: usize,
    residual_flag: u8,
) -> Result<(Option<&[u8]>, usize), IoError> {
    match residual_flag {
        0x00 => Ok((None, flag_offset + 1)),
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
            let rdata_end = rlen_end + rlen;
            if data.len() < rdata_end {
                return Err(IoError::Truncated {
                    needed: rdata_end,
                    got: data.len(),
                });
            }
            let rdata = data.get(rlen_end..rdata_end).ok_or(IoError::Truncated {
                needed: rdata_end,
                got: data.len(),
            })?;
            Ok((Some(rdata), rdata_end))
        }
        got => Err(IoError::Decode(
            tinyquant_core::errors::CodecError::InvalidResidualFlag { got },
        )),
    }
}
