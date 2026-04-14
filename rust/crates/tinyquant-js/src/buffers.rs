//! Typed-array bridging between JS (`Float32Array`) and Rust (`&[f32]`).
//!
//! Phase 25.2 scope: only what `Codec::compress` / `Codec::decompress`
//! on a single vector need. Batch paths (`compress_batch`,
//! `decompress_batch`) land in Phase 25.3.
//!
//! napi-rs v2 exposes `Float32Array` whose underlying storage is owned
//! by the V8 heap. We deliberately copy the input into a `Vec<f32>`
//! rather than holding a reference to the V8-owned buffer across the
//! `Codec::compress` call, because:
//!
//! 1. `allow_threads`-style GIL release doesn't exist in N-API; the
//!    call already runs on the main JS thread, so a copy is cheap
//!    relative to the SHA-256 + QR work that follows.
//! 2. Holding a `&Float32Array` across a potential future
//!    `AsyncTask` boundary is unsound because the buffer can be
//!    detached from the JS side.
//!
//! When Phase 25.3 wires `Codec.compressBatch` → `AsyncTask<...>`,
//! the input will already be owned (via a dedicated copy helper), so
//! the shape here generalizes cleanly.

use napi::bindgen_prelude::Float32Array;

/// Copy a JS `Float32Array` into an owned `Vec<f32>`.
///
/// `as_ref()` on a `Float32Array` returns `&[f32]` over the V8-owned
/// buffer; we immediately `.to_vec()` to sever the tie so the caller
/// can pass the buffer to `tinyquant-core` without lifetime issues.
pub(crate) fn float32_array_to_vec(arr: &Float32Array) -> Vec<f32> {
    let slice: &[f32] = arr.as_ref();
    slice.to_vec()
}

/// Wrap an owned `Vec<f32>` as a `Float32Array` for return to JS.
///
/// napi-rs v2's `Float32Array::new(Vec<f32>)` transfers ownership
/// into a fresh V8 `ArrayBuffer`, so no post-return copy is needed
/// on the JS side — `Float32Array.buffer` points at the Rust-allocated
/// memory until V8 collects it.
pub(crate) fn vec_to_float32_array(buf: Vec<f32>) -> Float32Array {
    Float32Array::new(buf)
}
