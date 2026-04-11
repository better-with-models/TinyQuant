//! Encode a `CompressedVector` to its binary wire format.

use crate::compressed_vector::{header::encode_header, pack::pack_indices};
use tinyquant_core::codec::CompressedVector;

/// Serialize `cv` to a `Vec<u8>` in the Level-1 wire format.
///
/// Layout: 70-byte header | packed indices | `residual_flag` | [`residual_length` u32 LE | residual bytes]
pub fn to_bytes(cv: &CompressedVector) -> Vec<u8> {
    let dim = cv.dimension() as usize;
    let bw = cv.bit_width();
    let packed_len = (dim * bw as usize + 7) / 8;
    // 1 flag + optional (4 length bytes + residual data)
    let residual_overhead = cv.residual().map_or(0, |r| 5 + r.len());
    let total = 70 + packed_len + 1 + residual_overhead;

    let mut out = Vec::with_capacity(total);
    encode_header(&mut out, cv.config_hash(), cv.dimension(), bw);

    // Pack indices
    let mut packed = vec![0u8; packed_len];
    pack_indices(cv.indices(), bw, &mut packed);
    out.extend_from_slice(&packed);

    // Residual section
    match cv.residual() {
        None => out.push(0x00),
        Some(r) => {
            out.push(0x01);
            // dim is u32; residual len = 2*dim fits in u32 only when dim <= 2^31.
            // Embedding dimensions in practice are <4096, so truncation cannot occur.
            #[allow(clippy::cast_possible_truncation)]
            let rlen = r.len() as u32;
            out.extend_from_slice(&rlen.to_le_bytes());
            out.extend_from_slice(r);
        }
    }
    out
}
