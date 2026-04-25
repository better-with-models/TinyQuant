//! Bit-packing: unpacked u8 indices -> compact LSB-first byte stream.
//!
//! For `bit_width = 2`: indices 0,1,2,3 share byte 0; index 0 in bits 0-1, index 1 in bits 2-3, etc.
//! For `bit_width = 4`: indices 0,1 share byte 0; index 0 in bits 0-3, index 1 in bits 4-7.
//! For `bit_width = 8`: plain byte copy.

/// Pack `indices` (one u8 per element, values in `[0, 2^bit_width)`) into
/// the LSB-first packed representation. `packed.len()` must equal
/// `(indices.len() * bit_width as usize + 7) / 8`.
pub fn pack_indices(indices: &[u8], bit_width: u8, packed: &mut [u8]) {
    debug_assert_eq!(
        packed.len(),
        (indices.len() * bit_width as usize).div_ceil(8)
    );
    if bit_width == 8 {
        packed.copy_from_slice(indices);
        return;
    }
    // Clear output.
    for b in packed.iter_mut() {
        *b = 0;
    }
    let bw = bit_width as usize;
    for (i, &idx) in indices.iter().enumerate() {
        let bit_start = i * bw;
        let byte_lo = bit_start / 8;
        let bit_off = bit_start % 8;
        // Low bits of idx that fit in byte_lo
        let fits = 8 - bit_off;
        let mask_lo = (1u8 << bw.min(fits)) - 1;
        // SAFETY: byte_lo < packed.len() because bit_start < indices.len() * bw <= packed.len() * 8
        #[allow(clippy::indexing_slicing)]
        {
            packed[byte_lo] |= (idx & mask_lo) << bit_off;
        }
        if bw > fits {
            // Overflow into the next byte
            let byte_hi = byte_lo + 1;
            let shift = fits;
            let mask_hi = (1u8 << (bw - fits)) - 1;
            #[allow(clippy::indexing_slicing)]
            {
                packed[byte_hi] |= (idx >> shift) & mask_hi;
            }
        }
    }
}
