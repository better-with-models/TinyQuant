// shaders/residual_decode.wgsl
//
// Unpack fp16 pairs from `residual_u16` and add them in-place to `values`.
//
// Buffer layout mirrors residual_encode.wgsl:
//   residual_u16[k] encodes two fp16 residuals:
//     lo 16 bits  = residual_f16[2k]
//     hi 16 bits  = residual_f16[2k+1]
//
// After this pass: values[i] += f16_to_f32(residual_f16[i])
// Dispatch: ceil(arrayLength(residual_u16) / 256) workgroups.

@group(0) @binding(0) var<storage, read>       residual_u16: array<u32>;  // packed fp16 pairs
@group(0) @binding(1) var<storage, read_write> values:       array<f32>;  // dequantized values (modified in-place)

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pair_idx = gid.x;
    if (pair_idx >= arrayLength(&residual_u16)) { return; }

    let packed   = residual_u16[pair_idx];
    let n_values = arrayLength(&values);

    let idx0 = pair_idx * 2u;
    let idx1 = pair_idx * 2u + 1u;

    if (idx0 < n_values) {
        values[idx0] += f16_to_f32(packed & 0xFFFFu);
    }
    if (idx1 < n_values) {
        values[idx1] += f16_to_f32((packed >> 16u) & 0xFFFFu);
    }
}

// Convert a 16-bit IEEE 754 half-precision bit pattern to f32.
// Handles ±zero, subnormal f16, normal, ±inf, NaN.
fn f16_to_f32(h: u32) -> f32 {
    let sign     = (h >> 15u) & 0x1u;
    let h_exp    = (h >> 10u) & 0x1Fu;
    let mantissa = h & 0x3FFu;

    // ±zero
    if (h_exp == 0u && mantissa == 0u) {
        return bitcast<f32>(sign << 31u);
    }
    // inf or NaN
    if (h_exp == 31u) {
        return bitcast<f32>((sign << 31u) | 0x7F800000u | (mantissa << 13u));
    }
    // subnormal f16 (h_exp == 0, mantissa != 0):
    //   value = (-1)^sign × 2^(−14) × (mantissa / 1024)
    //   f32 biased exponent: -14 + 127 = 113; mantissa left-shifted by 13.
    if (h_exp == 0u) {
        return bitcast<f32>((sign << 31u) | (113u << 23u) | (mantissa << 13u));
    }
    // normal f16 → normal f32
    let f_exp  = h_exp + 127u - 15u;
    let f_bits = (sign << 31u) | (f_exp << 23u) | (mantissa << 13u);
    return bitcast<f32>(f_bits);
}
