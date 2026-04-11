//! `TinyQuant` workspace task runner.
//!
//! Run from the `rust/` directory: `cargo xtask <command>`
//!
//! Commands:
//!
//! * `fmt`      — Format all Rust sources
//! * `lint`     — Run clippy with `-D warnings`
//! * `test`     — Run all workspace tests
//! * `fixtures` — Regenerate test fixtures (see subcommands below)
//! * `help`     — Print usage
//!
//! Fixture subcommands:
//!
//! * `fixtures refresh-hashes`   — Run
//!   `python scripts/generate_rust_fixtures.py hashes` to refresh
//!   `config_hashes.json` from the Python reference.
//! * `fixtures refresh-rotation` — Invoke the
//!   `dump_rotation_fixture` example binary for the canonical
//!   `(seed, dim)` gold set and overwrite the `.f64.bin` files.
//! * `fixtures refresh-codebook` — Run
//!   `python scripts/generate_rust_fixtures.py codebook` to refresh
//!   the 10 000 × 64 training corpus and the Python-trained
//!   `expected_bw{2,4,8}_seed42.f32.bin` codebook entries.
//! * `fixtures refresh-quantize` — Run
//!   `python scripts/generate_rust_fixtures.py quantize` to refresh
//!   the 10 000-value quantize corpus and the expected per-bit-width
//!   `u8` index files. Requires the codebook fixtures to be present.
//! * `fixtures refresh-residual` — Run
//!   `python scripts/generate_rust_fixtures.py residual` to refresh
//!   the 1 000 × 64 f32 original/reconstructed corpora and the
//!   expected fp16 residual byte fixture.
//! * `fixtures refresh-all`      — Run all of the above in sequence.
#![deny(warnings, clippy::all, clippy::pedantic)]

use std::path::{Path, PathBuf};
use std::process::{self, Command};

/// Canonical rotation fixtures captured under
/// `rust/crates/tinyquant-core/tests/fixtures/rotation/`.
const ROTATION_GOLD_SET: &[(u64, u32)] = &[(42, 64), (42, 768)];
const ROTATION_FIXTURE_DIR: &str = "crates/tinyquant-core/tests/fixtures/rotation";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let task = args.get(1).map(String::as_str);
    match task {
        Some("fmt") => run("cargo", &["fmt", "--manifest-path", "Cargo.toml", "--all"]),
        Some("lint") => run(
            "cargo",
            &[
                "clippy",
                "--manifest-path",
                "Cargo.toml",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        Some("test") => run(
            "cargo",
            &["test", "--manifest-path", "Cargo.toml", "--workspace"],
        ),
        Some("fixtures") => fixtures(args.get(2).map(String::as_str)),
        Some("help") | None => print_help(),
        Some(t) => {
            eprintln!("unknown task: {t}");
            process::exit(1);
        }
    }
}

fn fixtures(sub: Option<&str>) {
    match sub {
        Some("refresh-hashes") => refresh_hashes(),
        Some("refresh-rotation") => refresh_rotation(),
        Some("refresh-codebook") => refresh_codebook(),
        Some("refresh-quantize") => refresh_quantize(),
        Some("refresh-residual") => refresh_residual(),
        Some("refresh-all") => {
            refresh_hashes();
            refresh_rotation();
            refresh_codebook();
            refresh_quantize();
            refresh_residual();
        }
        _ => {
            eprintln!(
                "usage: cargo xtask fixtures <refresh-hashes|refresh-rotation|\
                 refresh-codebook|refresh-quantize|refresh-residual|refresh-all>"
            );
            process::exit(1);
        }
    }
}

fn refresh_hashes() {
    let repo_root = repo_root();
    println!(
        "xtask fixtures: running generate_rust_fixtures.py hashes from {}",
        repo_root.display()
    );
    let status = Command::new("python")
        .arg("scripts/generate_rust_fixtures.py")
        .arg("hashes")
        .current_dir(&repo_root)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to spawn python: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

fn refresh_codebook() {
    let repo_root = repo_root();
    println!(
        "xtask fixtures: running generate_rust_fixtures.py codebook from {}",
        repo_root.display()
    );
    let status = Command::new("python")
        .args([
            "scripts/generate_rust_fixtures.py",
            "codebook",
            "--seed",
            "42",
            "--rows",
            "10000",
            "--cols",
            "64",
        ])
        .current_dir(&repo_root)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to spawn python: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

fn refresh_quantize() {
    let repo_root = repo_root();
    println!(
        "xtask fixtures: running generate_rust_fixtures.py quantize from {}",
        repo_root.display()
    );
    let status = Command::new("python")
        .args([
            "scripts/generate_rust_fixtures.py",
            "quantize",
            "--seed",
            "7",
            "--count",
            "10000",
            "--codebook-seed",
            "42",
        ])
        .current_dir(&repo_root)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to spawn python: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

fn refresh_rotation() {
    for (seed, dim) in ROTATION_GOLD_SET {
        let rel_path = format!("{ROTATION_FIXTURE_DIR}/seed_{seed}_dim_{dim}.f64.bin");
        println!("xtask fixtures: generating {rel_path}");
        let status = Command::new("cargo")
            .args([
                "run",
                "--manifest-path",
                "Cargo.toml",
                "-p",
                "tinyquant-core",
                "--example",
                "dump_rotation_fixture",
                "--features",
                "std",
                "--",
                &seed.to_string(),
                &dim.to_string(),
                &rel_path,
            ])
            .status()
            .unwrap_or_else(|e| {
                eprintln!("failed to spawn cargo: {e}");
                process::exit(1);
            });
        if !status.success() {
            process::exit(status.code().unwrap_or(1));
        }
    }
}

fn repo_root() -> PathBuf {
    // xtask is invoked from `rust/`; the repository root is the parent.
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("failed to read cwd: {e}");
        process::exit(1);
    });
    parent_or_self(&cwd)
}

fn parent_or_self(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => path.to_path_buf(),
    }
}

fn run(prog: &str, args: &[&str]) {
    let status = Command::new(prog).args(args).status().unwrap_or_else(|e| {
        eprintln!("failed to spawn `{prog}`: {e}");
        process::exit(1);
    });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

fn print_help() {
    println!("xtask — `TinyQuant` workspace task runner");
    println!();
    println!("Commands:");
    println!("  fmt       Format all Rust sources");
    println!("  lint      Run clippy with -D warnings");
    println!("  test      Run all workspace tests");
    println!(
        "  fixtures  Regenerate test fixtures (refresh-hashes | refresh-rotation | \
         refresh-codebook | refresh-quantize | refresh-all)"
    );
    println!("  help      Print this message (default)");
}
