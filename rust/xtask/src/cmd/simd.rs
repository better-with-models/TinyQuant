//! `cargo xtask simd` subcommands (Phase 20).

use std::process;

/// CI job names that must appear in `rust-ci.yml` for the Phase 20
/// SIMD target-feature matrix.
const SIMD_CI_JOBS: &[&str] = &[
    "simd-scalar",
    "simd-avx2",
    "simd-neon",
    "miri-scalar",
    "simd-avx2-darwin",
    "simd-neon-darwin",
    "simd-avx2-windows",
];

pub fn run(sub: Option<&str>) {
    if let Some("audit") = sub {
        audit();
    } else {
        eprintln!("usage: cargo xtask simd <audit>");
        process::exit(1);
    }
}

fn audit() {
    // xtask lives at rust/xtask/; the workflow lives at <repo-root>/.github/workflows/
    let workflow = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("xtask must be under rust/xtask/")
        .join(".github")
        .join("workflows")
        .join("rust-ci.yml");
    println!(
        "xtask simd audit: checking {} for {} expected CI jobs",
        workflow.display(),
        SIMD_CI_JOBS.len()
    );
    let contents = match std::fs::read_to_string(&workflow) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {e}", workflow.display());
            process::exit(1);
        }
    };
    let mut missing: Vec<&str> = Vec::new();
    for &job in SIMD_CI_JOBS {
        // Check for the job key in YAML form (two-space indent + job-name + colon)
        if !contents.contains(&format!("  {job}:")) {
            missing.push(job);
        }
    }
    if missing.is_empty() {
        println!("xtask simd audit: OK ({} jobs found)", SIMD_CI_JOBS.len());
    } else {
        eprintln!("xtask simd audit: MISSING jobs: {missing:?}");
        process::exit(1);
    }
}
