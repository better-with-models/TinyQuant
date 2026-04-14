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
//! * `bench`    — Benchmark budget commands (see subcommands below)
//! * `bench-budget` — Phase 22.D alias for `bench --check-against main`
//! * `check-matrix-sync` — Phase 22.D: diff the CLI smoke matrix in the
//!   plan doc against `rust-release.yml`, and assert that all four
//!   `publish-*` jobs in `rust-release.yml` share a byte-identical
//!   `if:` guard expression (Phase 22.D code-quality follow-up M3)
//! * `docs`     — Documentation checks (see subcommands below)
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
//! * `fixtures refresh-codec`    — Run
//!   `python scripts/generate_rust_fixtures.py codec` to refresh the
//!   end-to-end codec byte-parity fixtures (indices, residual, decompressed)
//!   for bit-widths 2, 4, 8 against a 1 000 × 64 input corpus.
//! * `fixtures refresh-serialization` — Run
//!   `python scripts/generate_rust_fixtures.py serialization` to refresh
//!   the 10-case `CompressedVector` byte-parity fixtures for `tinyquant-io`.
//! * `fixtures refresh-corpus-file` — Run the `gen_corpus_fixture` example to
//!   regenerate `golden_100.tqcv` and `golden_100_indices.bin` in
//!   `crates/tinyquant-io/tests/fixtures/codec_file/`.
//! * `fixtures refresh-calibration` — Run `gen_openai_sample.py` to refresh the
//!   calibration fixture binaries and `manifest.json` (advisory; humans only).
//! * `fixtures refresh-all`      — Run all of the above in sequence.
//!
//! Bench subcommands:
//!
//! * `bench --capture-baseline <name>` — Run criterion and save a baseline JSON.
//! * `bench --check-against <name>`    — Compare current run against a baseline.
//! * `bench --validate`                — Assert `baselines/main.json` matches schema.
//! * `bench --diff <from> <to>`        — Compare two named baselines and print deltas.
//!
//! Docs subcommands:
//!
//! * `docs check-ci-parity` — Assert job names in the design doc appear in
//!   `rust-ci.yml` (Phase 14 Lesson L3 gate).
#![deny(warnings, clippy::all, clippy::pedantic)]

mod cmd {
    pub mod bench;
    pub mod guard_sync;
    pub mod matrix_sync;
    pub mod simd;
}

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
        Some("simd") => cmd::simd::run(args.get(2).map(String::as_str)),
        Some("bench") => cmd::bench::run(&args[2..]),
        // Phase 22.D: `bench-budget` is a convenience alias that runs the
        // budget check against the canonical `main` baseline. It exists so
        // the release workflow can call a single verb without having to
        // encode the flag incantation in YAML.
        Some("bench-budget") => cmd::bench::run(&["--check-against".to_owned(), "main".to_owned()]),
        Some("check-matrix-sync") => {
            // Both checks share a single CLI verb so the release workflow
            // only needs one step. Run them in sequence — matrix first
            // (docs ↔ release.yml), publish-guards second — and bail on
            // the first failure so the cause is obvious in CI logs.
            cmd::matrix_sync::run();
            cmd::guard_sync::run();
        }
        Some("docs") => docs(args.get(2).map(String::as_str)),
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
        Some("refresh-codec") => refresh_codec(),
        Some("refresh-serialization") => refresh_serialization(),
        Some("refresh-corpus-file") => refresh_corpus_file(),
        Some("refresh-calibration") => refresh_calibration(),
        Some("refresh-all") => {
            refresh_hashes();
            refresh_rotation();
            refresh_codebook();
            refresh_quantize();
            refresh_residual();
            refresh_codec();
            refresh_serialization();
            refresh_corpus_file();
            refresh_calibration();
        }
        _ => {
            eprintln!(
                "usage: cargo xtask fixtures <refresh-hashes|refresh-rotation|\
                 refresh-codebook|refresh-quantize|refresh-residual|refresh-codec|\
                 refresh-serialization|refresh-corpus-file|refresh-calibration|refresh-all>"
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

fn refresh_residual() {
    let repo_root = repo_root();
    println!(
        "xtask fixtures: running generate_rust_fixtures.py residual from {}",
        repo_root.display()
    );
    let status = Command::new("python")
        .args([
            "scripts/generate_rust_fixtures.py",
            "residual",
            "--seed",
            "19",
            "--rows",
            "1000",
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

fn refresh_codec() {
    // Rust and Python use different RNG algorithms for the rotation matrix
    // (ChaCha20 vs PCG64), so the binary fixtures are generated from Rust
    // to lock in deterministic behaviour.  Python generates only the
    // fidelity_manifest.json.
    let training_rel = "crates/tinyquant-core/tests/fixtures/codebook/training_n10000_d64.f32.bin";
    let out_rel = "crates/tinyquant-core/tests/fixtures/codec";

    println!("xtask fixtures: generating codec binary fixtures via dump_codec_fixture");
    let status = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "Cargo.toml",
            "-p",
            "tinyquant-core",
            "--example",
            "dump_codec_fixture",
            "--features",
            "std",
            "--",
            "11",   // input-seed
            "42",   // codec-seed
            "1000", // rows
            "64",   // cols
            training_rel,
            out_rel,
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to spawn cargo: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }

    // Python generates only the fidelity manifest (quality metrics from
    // the Python codec — separate algorithm, separate quality baseline).
    let repo_root = repo_root();
    println!(
        "xtask fixtures: generating fidelity_manifest.json from {}",
        repo_root.display()
    );
    let status = Command::new("python")
        .args([
            "scripts/generate_rust_fixtures.py",
            "codec",
            "--input-seed",
            "11",
            "--codec-seed",
            "42",
            "--rows",
            "1000",
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

fn refresh_corpus_file() {
    println!("xtask fixtures: generating golden corpus file fixtures");
    let status = Command::new("cargo")
        .args([
            "run",
            "--manifest-path",
            "Cargo.toml",
            "-p",
            "tinyquant-io",
            "--example",
            "gen_corpus_fixture",
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

fn refresh_serialization() {
    let repo_root = repo_root();
    println!(
        "xtask fixtures: running generate_rust_fixtures.py serialization from {}",
        repo_root.display()
    );
    let status = Command::new("python")
        .args(["scripts/generate_rust_fixtures.py", "serialization"])
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

fn refresh_calibration() {
    let repo_root = repo_root();
    println!(
        "xtask fixtures: running gen_openai_sample.py from {}",
        repo_root.display()
    );
    let status = Command::new("python")
        .args([
            "scripts/calibration/gen_openai_sample.py",
            "--seed",
            "42",
            "--out",
            "rust/crates/tinyquant-bench/fixtures/calibration",
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

fn docs(sub: Option<&str>) {
    if sub == Some("check-ci-parity") {
        docs_check_ci_parity();
    } else {
        eprintln!("usage: cargo xtask docs <check-ci-parity>");
        process::exit(1);
    }
}

// Job names from the Phase 21 plan that must exist in the workflow.
const REQUIRED_CI_JOBS: &[&str] = &["calibration", "bench-budget"];

/// Assert that CI job names mentioned in the design doc appear in `rust-ci.yml`.
///
/// Implements Phase 14 Lesson L3: prevents CI/design-doc drift.
fn docs_check_ci_parity() {
    let repo_root = repo_root();
    let ci_path = repo_root.join(".github/workflows/rust-ci.yml");
    let ci_text = std::fs::read_to_string(&ci_path).unwrap_or_else(|e| {
        eprintln!(
            "docs check-ci-parity: cannot read {}: {e}",
            ci_path.display()
        );
        process::exit(1);
    });

    let mut missing = Vec::new();
    for job in REQUIRED_CI_JOBS {
        if !ci_text.contains(&format!("{job}:")) {
            missing.push(*job);
        }
    }
    if missing.is_empty() {
        println!(
            "docs check-ci-parity: all required job names present in {} ✓",
            ci_path.display()
        );
    } else {
        eprintln!(
            "docs check-ci-parity: the following job names are missing from {}:",
            ci_path.display()
        );
        for job in &missing {
            eprintln!("  - {job}");
        }
        process::exit(1);
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
         refresh-codebook | refresh-quantize | refresh-residual | refresh-codec | \
         refresh-serialization | refresh-corpus-file | refresh-calibration | refresh-all)"
    );
    println!(
        "  bench     Benchmark budget (--capture-baseline | --check-against | --diff | --validate)"
    );
    println!("  bench-budget        Alias for `bench --check-against main` (Phase 22.D)");
    println!(
        "  check-matrix-sync   Diff CLI smoke matrix in plan vs rust-release.yml + assert \
         publish-* guards byte-identical (Phase 22.D)"
    );
    println!("  docs      Documentation checks (check-ci-parity)");
    println!("  simd      SIMD framework tasks (audit)");
    println!("  help      Print this message (default)");
}
