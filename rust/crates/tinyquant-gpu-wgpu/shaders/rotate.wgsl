// shaders/rotate.wgsl
// Batched matrix–vector multiply: output[row, col] = sum_k input[row, k] * rotation[k, col].
// Implements the forward rotation step of TinyQuant compression.
struct Dims { rows: u32, cols: u32 }

@group(0) @binding(0) var<uniform>             dims:     Dims;
@group(0) @binding(1) var<storage, read>       rotation: array<f32>;  // cols×cols
@group(0) @binding(2) var<storage, read>       input:    array<f32>;  // rows×cols
@group(0) @binding(3) var<storage, read_write> output:   array<f32>;  // rows×cols

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let row = gid.y;
    let col = gid.x;
    if (row >= dims.rows || col >= dims.cols) { return; }
    var acc: f32 = 0.0;
    for (var k: u32 = 0u; k < dims.cols; k++) {
        acc += input[row * dims.cols + k] * rotation[k * dims.cols + col];
    }
    output[row * dims.cols + col] = acc;
}
