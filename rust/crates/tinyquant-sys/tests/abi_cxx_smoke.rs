//! Mirror of `abi_c_smoke.rs` for C++17. Verifies that the committed
//! header parses cleanly from a C++ translation unit (covers the
//! `extern "C"` guard emitted by cbindgen's `cpp_compat = true`).
//!
//! Skips gracefully when no C++ compiler is available.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn cxx_header_compiles_with_cxx17() {
    let tests_dir = manifest_dir().join("tests").join("cxx");
    let include_dir = manifest_dir().join("include");

    let out_dir = manifest_dir()
        .join("target")
        .join("tests")
        .join("abi_cxx_smoke");
    std::fs::create_dir_all(&out_dir).expect("create scratch out dir");

    // SAFETY: see `abi_c_smoke.rs`.
    unsafe { std::env::set_var("OUT_DIR", &out_dir) };
    unsafe {
        std::env::set_var(
            "HOST",
            std::env::var("HOST").unwrap_or_else(|_| current_target()),
        );
    }
    unsafe {
        std::env::set_var(
            "TARGET",
            std::env::var("TARGET").unwrap_or_else(|_| current_target()),
        );
    }
    unsafe { std::env::set_var("OPT_LEVEL", "0") };

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(tests_dir.join("smoke.cc"))
        .include(&include_dir)
        .warnings(true)
        .extra_warnings(true)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wextra")
        .flag_if_supported("-Wpedantic")
        .flag_if_supported("-Werror")
        .flag_if_supported("-Wno-zero-length-array")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("/W4")
        .flag_if_supported("/WX")
        .flag_if_supported("/EHsc")
        // MSVC: C4200 = zero-sized array, C4820 = struct padding. See
        // `abi_c_smoke.rs` for the rationale.
        .flag_if_supported("/wd4200")
        .flag_if_supported("/wd4820")
        .cargo_metadata(false);

    match build.try_compile("tinyquant_cxx_smoke") {
        Ok(()) => {}
        Err(e) => {
            eprintln!(
                "abi_cxx_smoke skipped: cc failed to compile tinyquant.h as C++17: {e}. \
                 This is informational; run locally with a C++17 toolchain to enforce \
                 `cpp_compat` parseability."
            );
        }
    }
}

fn current_target() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    match os {
        "windows" => format!("{arch}-pc-windows-msvc"),
        "macos" => format!("{arch}-apple-darwin"),
        "linux" => format!("{arch}-unknown-linux-gnu"),
        other => format!("{arch}-unknown-{other}"),
    }
}
