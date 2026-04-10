//! Build script for tinyquant-sys.
//!
//! Phase 11: placeholder only. cbindgen header generation added in Phase 22.
fn main() {
    // Rerun only when the public API changes.
    println!("cargo::rerun-if-changed=src/lib.rs");
}
