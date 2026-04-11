//! Bit-unpacking: compact LSB-first byte stream -> unpacked u8 indices.

/// Unpack `packed` bytes into `out` (one u8 per element). The caller must
/// ensure `out.len() == dim` and `packed.len() == (dim * bit_width as usize + 7) / 8`.
pub fn unpack_indices(packed: &[u8], dim: usize, bit_width: u8, out: &mut [u8]) {
    debug_assert_eq!(out.len(), dim);
    debug_assert_eq!(packed.len(), (dim * bit_width as usize + 7) / 8);
    if bit_width == 8 {
        out.copy_from_slice(packed);
        return;
    }
    let bw = bit_width as usize;
    let mask = (1u8 << bw) - 1;
    for (i, slot) in out.iter_mut().enumerate().take(dim) {
        let bit_start = i * bw;
        let byte_lo = bit_start / 8;
        let bit_off = bit_start % 8;
        let fits = 8 - bit_off;
        // SAFETY: byte_lo < packed.len() because bit_start < dim * bw <= packed.len() * 8
        #[allow(clippy::indexing_slicing)]
        let lo_bits = (packed[byte_lo] >> bit_off) & mask;
        let val = if bw > fits {
            let byte_hi = byte_lo + 1;
            #[allow(clippy::indexing_slicing)]
            let hi_bits = packed[byte_hi] & ((1u8 << (bw - fits)) - 1);
            lo_bits | (hi_bits << fits)
        } else {
            lo_bits
        };
        *slot = val;
    }
}
