//! Build script for `tinyquant-cli`.
//!
//! Captures build-time metadata so the `info` subcommand (Phase 22
//! plan §Step 15) can print it at runtime without a runtime
//! dependency on `git` or `rustc`.
//!
//! Emits four `cargo:rustc-env=` directives:
//!
//! - `TINYQUANT_GIT_COMMIT` — short SHA of the current git HEAD, or
//!   `"unknown"` when the build happens outside a git worktree (for
//!   example, inside the vendored crates.io tarball).
//! - `TINYQUANT_RUSTC_VERSION` — output of `rustc --version`.
//! - `TINYQUANT_TARGET` — the `TARGET` Cargo passed to rustc.
//! - `TINYQUANT_PROFILE` — `"release"` or `"debug"` from the Cargo
//!   `PROFILE` env var.
//!
//! The script is deliberately forgiving: every probe falls back to
//! `"unknown"` on failure so a builder without `git` or a detached
//! working copy still compiles. The `cargo:rerun-if-changed` lines
//! keep the environment stamps fresh across normal development.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=TINYQUANT_GIT_COMMIT");

    // Rerun on HEAD movement when building from a live checkout. These
    // files may not exist (crates.io tarball, git-less CI), so we
    // suppress errors silently.
    for path in &[".git/HEAD", ".git/refs/heads", "../../.git/HEAD"] {
        println!("cargo:rerun-if-changed={path}");
    }

    let git_commit = env_or("TINYQUANT_GIT_COMMIT")
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=12", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=TINYQUANT_GIT_COMMIT={git_commit}");

    let rustc_version = Command::new(env_or("RUSTC").unwrap_or_else(|| "rustc".to_owned()))
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=TINYQUANT_RUSTC_VERSION={rustc_version}");

    let target = env_or("TARGET").unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=TINYQUANT_TARGET={target}");

    let profile = env_or("PROFILE").unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=TINYQUANT_PROFILE={profile}");
}

fn env_or(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}
