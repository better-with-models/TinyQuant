// shaders/cosine_topk.wgsl
//
// Single-pass cosine-similarity scoring kernel for GPU-resident corpus search.
//
// Each GPU thread scores one corpus row against the query vector and writes
// the result to `scores[row]`.  Host-side top-k selection follows the GPU
// pass, reading back all n_rows floats and sorting on CPU.
//
// Pre-normalisation contract
// --------------------------
// Vectors must be pre-normalised to unit length before upload.  Under that
// contract cosine_similarity(q, r) == dot(q, r), which this kernel computes.
// If vectors are *not* pre-normalised the scores will be unnormalised dot
// products; callers are responsible for normalisation.
//
// Dispatch pattern
// ----------------
// @workgroup_size(256); dispatch ceil(n_rows / 256) workgroups.
// Threads with row >= n_rows return immediately (tail guard).

struct Dims {
    n_rows : u32,   // corpus row count
    dim    : u32,   // embedding dimension
    top_k  : u32,   // reserved — host performs the selection, not this shader
    _pad   : u32,   // align struct to 16 bytes for uniform binding
}

@group(0) @binding(0) var<uniform>             dims   : Dims;
@group(0) @binding(1) var<storage, read>       corpus : array<f32>; // n_rows × dim, row-major
@group(0) @binding(2) var<storage, read>       query  : array<f32>; // dim elements
@group(0) @binding(3) var<storage, read_write> scores : array<f32>; // n_rows outputs

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let row = gid.x;
    if row >= dims.n_rows { return; }

    let offset = row * dims.dim;
    var dot: f32 = 0.0;
    for (var d: u32 = 0u; d < dims.dim; d = d + 1u) {
        dot = dot + corpus[offset + d] * query[d];
    }
    scores[row] = dot;
}
