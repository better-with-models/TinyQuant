// shaders/quantize.wgsl
// For each element: find the codebook entry with minimum squared distance.
// Uses a 4-bit codebook (16 entries). Input and codebook are f32 arrays.
struct Dims { n_elements: u32, n_entries: u32 }

@group(0) @binding(0) var<uniform>             dims:     Dims;
@group(0) @binding(1) var<storage, read>       codebook: array<f32>;
@group(0) @binding(2) var<storage, read>       input:    array<f32>;
@group(0) @binding(3) var<storage, read_write> indices:  array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= dims.n_elements) { return; }
    let val = input[i];
    var best_idx: u32  = 0u;
    var best_dist: f32 = 1e38;
    for (var k: u32 = 0u; k < dims.n_entries; k++) {
        let d = val - codebook[k];
        let dist = d * d;
        if (dist < best_dist) { best_dist = dist; best_idx = k; }
    }
    indices[i] = best_idx;
}
