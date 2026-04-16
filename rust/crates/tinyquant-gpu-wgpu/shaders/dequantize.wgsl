// shaders/dequantize.wgsl
// Index-to-entry lookup: out[i] = codebook_entries[indices[i]].
@group(0) @binding(0) var<storage, read>       entries: array<f32>;  // codebook entries
@group(0) @binding(1) var<storage, read>       indices: array<u32>;  // quantized indices
@group(0) @binding(2) var<storage, read_write> out:     array<f32>;  // reconstructed values

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&indices)) {
        out[i] = entries[indices[i]];
    }
}
