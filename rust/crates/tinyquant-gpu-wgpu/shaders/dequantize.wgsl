// shaders/dequantize.wgsl
// Index-to-entry lookup: out[i] = codebook_entries[indices[i]].
@group(0) @binding(0) var<storage, read>       entries: array<f32>;  // codebook entries
@group(0) @binding(1) var<storage, read>       indices: array<u32>;  // quantized indices
@group(0) @binding(2) var<storage, read_write> out:     array<f32>;  // reconstructed values

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&indices)) {
        // Clamp the index to [0, n_entries - 1] to guard against a malformed or
        // corrupted CompressedVector carrying an out-of-range index.  GPU
        // out-of-bounds reads are platform-defined (likely zero on wgpu/Vulkan
        // but not guaranteed), so we clamp rather than rely on that behaviour.
        let safe_idx = min(indices[i], arrayLength(&entries) - 1u);
        out[i] = entries[safe_idx];
    }
}
