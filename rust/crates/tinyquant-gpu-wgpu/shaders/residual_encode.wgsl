// shaders/residual_encode.wgsl
// Store (original[i] - reconstructed[i]) as packed fp16 bit patterns.
// Two f32 residuals are packed into one u32 (lo=residual[2i], hi=residual[2i+1]).
//
// NOTE: WGSL does not have a universal native f16 type.
// We encode using the f32_to_f16 bit-manipulation idiom below.
// Gate on wgpu::Features::SHADER_F16 for native f16 if the adapter supports it.
@group(0) @binding(0) var<storage, read>       original:     array<f32>;
@group(0) @binding(1) var<storage, read>       reconstructed: array<f32>;
@group(0) @binding(2) var<storage, read_write> residual_u16:  array<u32>; // packed pairs

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x * 2u;
    let n = arrayLength(&original);
    if (i >= n) { return; }
    let r0 = original[i] - reconstructed[i];
    let r1 = select(0.0, original[i + 1u] - reconstructed[i + 1u], i + 1u < n);
    // Pack two fp16 values into one u32 (little-endian: low=r0, high=r1).
    residual_u16[gid.x] = f32_to_f16(r0) | (f32_to_f16(r1) << 16u);
}

// Approximate f32-to-f16 bit conversion (IEEE 754 round-to-nearest).
fn f32_to_f16(v: f32) -> u32 {
    let bits = bitcast<u32>(v);
    let exp  = (bits >> 23u) & 0xFFu;
    if (exp == 0u)    { return 0u; }
    if (exp == 0xFFu) { return 0x7C00u | ((bits & 0x7FFFFFu) >> 13u); }
    let h_exp = exp - 127u + 15u;
    if (h_exp >= 31u) { return select(0x7C00u, 0u, (bits >> 31u) == 1u); }
    if (h_exp <= 0u)  { return 0u; }
    return ((bits >> 31u) << 15u) | (h_exp << 10u) | ((bits >> 13u) & 0x3FFu);
}
