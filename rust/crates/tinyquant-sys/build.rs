//! Build script for `tinyquant-sys`.
//!
//! Regenerates `include/tinyquant.h` via cbindgen on every
//! `cargo build -p tinyquant-sys`. The generated header is committed to
//! source control; CI fails on any non-empty diff under
//! `rust/crates/tinyquant-sys/include/`.
//!
//! See `docs/plans/rust/phase-22-pyo3-cabi-release.md` §cbindgen drift
//! guard for the rationale.

use std::path::PathBuf;

fn main() {
    let crate_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo");
    let crate_dir = PathBuf::from(crate_dir);
    let include_dir = crate_dir.join("include");
    std::fs::create_dir_all(&include_dir).expect("create include/ directory");

    let config_path = crate_dir.join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path)
        .unwrap_or_else(|e| panic!("failed to load {}: {e}", config_path.display()));

    let header_path = include_dir.join("tinyquant.h");

    // NOTE: cbindgen's `with_crate` runs `cargo metadata`, which in this
    // workspace fails because the pgvector crate transitively references
    // a registry package that requires a newer Cargo than our MSRV 1.81.
    // `with_src` side-steps metadata by enumerating the Rust sources
    // directly — cbindgen parses each file, follows `use crate::...` at
    // syntactic level, and never invokes cargo. Keep this list in sync
    // with `src/`.
    // cbindgen parses `lib.rs` and follows `pub mod` declarations
    // syntactically, so we only feed it the crate root. Listing each
    // module explicitly would emit every function twice (once at the
    // module site, once at the re-export). See the duplicate-decl
    // investigation notes in the Phase 22 plan.
    let src = crate_dir.join("src");
    let builder = cbindgen::Builder::new()
        .with_config(config)
        .with_src(src.join("lib.rs"));

    match builder.generate() {
        Ok(bindings) => {
            bindings.write_to_file(&header_path);
            // cbindgen does NOT expand `@version@` in the configured
            // `header = "..."` block — that token is only substituted by
            // the (deprecated) `with_crate` code-path via `cargo
            // metadata`. We use `with_src` to bypass `cargo metadata`
            // (MSRV incompatibility — see the build script header
            // comment), so we do the substitution ourselves here using
            // the build-time value of `CARGO_PKG_VERSION`.
            //
            // Keeping this step in `build.rs` means the header is always
            // regenerated in lock-step with the crate version; a
            // compile-time `const` check in `src/lib.rs`
            // (`TINYQUANT_H_VERSION` vs `env!("CARGO_PKG_VERSION")`)
            // catches any drift that slips past here.
            let version = std::env::var("CARGO_PKG_VERSION")
                .expect("CARGO_PKG_VERSION is always set by cargo");
            let contents = std::fs::read_to_string(&header_path)
                .unwrap_or_else(|e| panic!("read {}: {e}", header_path.display()));
            let substituted = contents.replace("@version@", &version);
            if substituted != contents {
                std::fs::write(&header_path, substituted)
                    .unwrap_or_else(|e| panic!("write {}: {e}", header_path.display()));
            }
        }
        Err(e) => {
            // During `cargo publish` we may be invoked without a usable
            // Rust source tree (e.g. crates.io repackaging). In that
            // case emit a warning instead of failing the build.
            println!(
                "cargo:warning=cbindgen failed to generate {}: {e}",
                header_path.display()
            );
        }
    }

    // Rerun whenever anything in src/ or the cbindgen config changes.
    // Also rerun when CARGO_PKG_VERSION changes so the post-processing
    // step above catches a crate version bump even when no source file
    // under src/ was touched.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
}
