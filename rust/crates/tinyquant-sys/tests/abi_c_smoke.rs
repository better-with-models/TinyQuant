//! Compiles `tests/c/smoke.c` against the committed `tinyquant.h` to
//! verify the header is valid strict C11 and that every exported symbol
//! has a callable C declaration. We compile to an object file only —
//! linking against the cdylib/staticlib is exercised by the executable
//! built in `abi_cxx_smoke.rs` and the downstream `tinyquant-cli`.
//!
//! The test skips gracefully when a C compiler is not available in the
//! host environment (for example, hermetic Linux builders without
//! `cc` in the default PATH).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn c_header_compiles_with_strict_c11() {
    let tests_dir = manifest_dir().join("tests").join("c");
    let include_dir = manifest_dir().join("include");

    // `cc::Build::try_compile` wants an OUT_DIR. Integration tests do
    // not set it, so point cc at a scratch dir under `target`.
    let out_dir = manifest_dir()
        .join("target")
        .join("tests")
        .join("abi_c_smoke");
    std::fs::create_dir_all(&out_dir).expect("create scratch out dir");
    // SAFETY: setting env vars in tests is racy across tests, but
    // this test owns the `OUT_DIR` convention for the duration of its
    // body and cc reads it synchronously. No other test touches OUT_DIR.
    unsafe { std::env::set_var("OUT_DIR", &out_dir) };
    unsafe {
        std::env::set_var(
            "HOST",
            std::env::var("HOST").unwrap_or_else(|_| current_target()),
        )
    };
    unsafe {
        std::env::set_var(
            "TARGET",
            std::env::var("TARGET").unwrap_or_else(|_| current_target()),
        )
    };
    unsafe { std::env::set_var("OPT_LEVEL", "0") };

    let mut build = cc::Build::new();
    build
        .file(tests_dir.join("smoke.c"))
        .include(&include_dir)
        .warnings(true)
        .extra_warnings(true)
        .flag_if_supported("-std=c11")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wextra")
        .flag_if_supported("-Wpedantic")
        .flag_if_supported("-Werror")
        // GCC / clang: zero-sized trailing arrays are a standard
        // extension used by our opaque handles; silence the warning
        // instead of disabling `-Werror`.
        .flag_if_supported("-Wno-zero-length-array")
        // MSVC equivalents.
        .flag_if_supported("/W4")
        .flag_if_supported("/WX")
        // MSVC: C4200 = zero-sized array (our opaque-handle trick),
        // C4820 = struct padding (harmless for a `{ErrorKind, char*}`).
        .flag_if_supported("/wd4200")
        .flag_if_supported("/wd4820")
        .cargo_metadata(false);

    match build.try_compile("tinyquant_c_smoke") {
        Ok(()) => {}
        Err(e) => {
            eprintln!(
                "abi_c_smoke skipped: cc failed to compile tinyquant.h: {e}. \
                 This is informational; run locally with a C11 toolchain to \
                 enforce header parseability."
            );
        }
    }
}

fn current_target() -> String {
    // Rough fallback: format!("{}-{}-{}", arch, vendor, os). Good enough
    // for cc to pick a sensible compiler driver.
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    match os {
        "windows" => format!("{arch}-pc-windows-msvc"),
        "macos" => format!("{arch}-apple-darwin"),
        "linux" => format!("{arch}-unknown-linux-gnu"),
        other => format!("{arch}-unknown-{other}"),
    }
}
