//! JavaScript/TypeScript bindings for TinyQuant (napi-rs).
//!
//! This crate is a façade over `tinyquant-core`; the first slice
//! (Phase 25.1) only exposes `version()` so the wheel tooling can be
//! validated end-to-end before the value-object surface lands in
//! Phase 25.2.
use napi_derive::napi;

/// Package version, forwarded from the workspace Cargo.toml.
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
