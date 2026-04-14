//! `tinyquant info` — print build + runtime metadata.
//!
//! Implements Phase 22 plan §Step 15. Prints eight labelled lines to
//! stdout:
//!
//! 1. `tinyquant <version>`
//! 2. `git commit:   <short-sha>`
//! 3. `rustc:        <rustc --version output>`
//! 4. `target:       <target triple>`
//! 5. `profile:      release|debug`
//! 6. `features:     <enabled cargo features>`
//! 7. `detected isa: Scalar|Avx2|Neon` (runtime ISA dispatch result)
//! 8. `cpu count:    <num_cpus::get()>`
//!
//! All of 2–5 come from `build.rs` via `cargo:rustc-env=`.

use anyhow::Result;

/// Entry point for `tinyquant info`.
///
/// Returns `Result<()>` for uniformity with the other `commands::*::run`
/// entry points — the underlying `println!` calls only fail on a
/// broken stdout pipe, which the stdlib surfaces as a panic rather
/// than a `Result`. Hence the unavoidable `unnecessary_wraps` allow.
///
/// # Errors
///
/// Propagates any I/O error from writing to stdout.
#[allow(clippy::unnecessary_wraps)]
pub fn run() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    println!("tinyquant version {version}");
    println!(
        "git commit:   {}",
        option_env!("TINYQUANT_GIT_COMMIT").unwrap_or("unknown")
    );
    println!(
        "rustc:        {}",
        option_env!("TINYQUANT_RUSTC_VERSION").unwrap_or("unknown")
    );
    println!(
        "target:       {}",
        option_env!("TINYQUANT_TARGET").unwrap_or("unknown")
    );
    println!(
        "profile:      {}",
        option_env!("TINYQUANT_PROFILE").unwrap_or("unknown")
    );
    println!("features:     {}", enabled_features().join(","));
    println!("detected isa: {}", detected_isa());
    println!("cpu count:    {}", num_cpus::get());
    Ok(())
}

/// List of enabled Cargo features at compile time — single source of
/// truth is this `cfg!` cascade, kept in sync with
/// `tinyquant-cli/Cargo.toml`.
fn enabled_features() -> Vec<&'static str> {
    let mut out = Vec::new();
    if cfg!(feature = "jemalloc") {
        out.push("jemalloc");
    }
    if cfg!(feature = "rayon") {
        out.push("rayon");
    }
    if cfg!(feature = "simd") {
        out.push("simd");
    }
    if cfg!(feature = "mmap") {
        out.push("mmap");
    }
    if cfg!(feature = "progress") {
        out.push("progress");
    }
    if cfg!(feature = "tracing-json") {
        out.push("tracing-json");
    }
    if out.is_empty() {
        out.push("<none>");
    }
    out
}

/// Human-readable representation of the runtime ISA pick.
///
/// Under `feature = "simd"` we route through
/// `tinyquant_core::codec::dispatch::current`, which caches the result
/// after the first probe. Without the `simd` feature the core crate
/// does not expose a `dispatch` module at all, so we report `Scalar`.
fn detected_isa() -> String {
    #[cfg(feature = "simd")]
    {
        format!("{:?}", tinyquant_core::codec::dispatch::current())
    }
    #[cfg(not(feature = "simd"))]
    {
        "Scalar".to_owned()
    }
}
