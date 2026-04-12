//! Xtask bench subcommand (Phase 21).
//!
//! # Subcommands
//!
//! | Command | Description |
//! |---------|-------------|
//! | `--capture-baseline <name>` | Run criterion and write a baseline JSON. |
//! | `--check-against <name>` | Compare current run against a committed baseline. |
//! | `--validate` | Validate `baselines/main.json` against `baselines/schema.json`. |
//!
//! # Baseline semantics
//!
//! A regression is detected when
//! `new_median > baseline_median * (budget_pct / 100)`.
//! The default `budget_pct` is **115** (15 % regression allowed).
//!
//! Baselines committed to `baselines/*.json` MUST be captured on a CI
//! runner, not a developer laptop, to avoid noise-induced flakes
//! (Phase 21 Risk R21.8).

use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

use serde_json::Value;

/// Entry point — parse `args` and dispatch.
pub fn run(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("--capture-baseline") => {
            let name = args.get(1).map_or("main", String::as_str);
            capture_baseline(name);
        }
        Some("--check-against") => {
            let name = args.get(1).map_or("main", String::as_str);
            check_against(name);
        }
        Some("--validate") => {
            validate_baseline("main");
        }
        _ => {
            eprintln!(
                "usage: cargo xtask bench \
                 <--capture-baseline <name>|--check-against <name>|--validate>"
            );
            process::exit(1);
        }
    }
}

// ── Paths ────────────────────────────────────────────────────────────────────

fn baseline_dir() -> PathBuf {
    // xtask runs from `rust/`; baselines live in the bench crate.
    PathBuf::from("crates/tinyquant-bench/baselines")
}

fn baseline_path(name: &str) -> PathBuf {
    baseline_dir().join(format!("{name}.json"))
}

fn schema_path() -> PathBuf {
    baseline_dir().join("schema.json")
}

/// Criterion output directory for the `batch_parallel` bench.
fn criterion_dir() -> PathBuf {
    PathBuf::from("target/criterion")
}

fn clear_criterion_cache() {
    let dir = criterion_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap_or_else(|e| {
            eprintln!("warning: failed to clear {}: {e}", dir.display());
        });
    }
}

// ── Validate ─────────────────────────────────────────────────────────────────

fn validate_baseline(name: &str) {
    let bl_path = baseline_path(name);
    let bl_text = fs::read_to_string(&bl_path).unwrap_or_else(|e| {
        eprintln!("xtask bench --validate: cannot read {}: {e}", bl_path.display());
        process::exit(1);
    });
    let baseline: Value = serde_json::from_str(&bl_text).unwrap_or_else(|e| {
        eprintln!("xtask bench --validate: {}: invalid JSON: {e}", bl_path.display());
        process::exit(1);
    });

    let schema_text = fs::read_to_string(schema_path()).unwrap_or_else(|e| {
        eprintln!("xtask bench --validate: cannot read schema.json: {e}");
        process::exit(1);
    });
    let schema: Value = serde_json::from_str(&schema_text).unwrap_or_else(|e| {
        eprintln!("xtask bench --validate: schema.json is invalid JSON: {e}");
        process::exit(1);
    });

    // Lightweight structural validation (no full JSON-Schema engine dep).
    let sv = baseline.get("schema_version").and_then(Value::as_i64).unwrap_or(-1);
    let expected_sv = schema
        .get("properties")
        .and_then(|p| p.get("schema_version"))
        .and_then(|sv| sv.get("const"))
        .and_then(Value::as_i64)
        .unwrap_or(1);

    if sv != expected_sv {
        eprintln!(
            "xtask bench --validate: schema_version={sv} != expected {expected_sv}"
        );
        process::exit(1);
    }

    for required in ["captured_at", "git_commit", "host", "bench_groups"] {
        if baseline.get(required).is_none() {
            eprintln!("xtask bench --validate: missing required field '{required}'");
            process::exit(1);
        }
    }

    println!("xtask bench --validate: {bl_path:?} is valid ✓");
}

// ── Capture baseline ──────────────────────────────────────────────────────────

fn capture_baseline(name: &str) {
    // Warn if running outside CI.
    let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    if !is_ci {
        eprintln!(
            "WARNING: capturing baseline outside CI. \
             Baselines committed to the repo MUST be captured on a CI runner \
             (Phase 21 Risk R21.8). This capture is for local diagnosis only."
        );
    }

    println!("xtask bench --capture-baseline {name}: running criterion …");
    clear_criterion_cache();
    let status = Command::new("cargo")
        .args([
            "bench",
            "--manifest-path",
            "Cargo.toml",
            "-p",
            "tinyquant-bench",
            "--bench",
            "batch_parallel",
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to run cargo bench: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }

    // Parse criterion output and build the baseline JSON.
    let groups = collect_criterion_results();
    if groups.is_empty() {
        eprintln!(
            "xtask bench --capture-baseline: no criterion estimates found in {}",
            criterion_dir().display()
        );
        process::exit(1);
    }

    let git_commit = git_short_sha();
    let rustc_ver = rustc_version();
    let (os, arch) = os_arch();

    let baseline = serde_json::json!({
        "schema_version": 1,
        "captured_at": now_utc(),
        "git_commit": git_commit,
        "host": {
            "os": os,
            "arch": arch,
            "rustc": rustc_ver
        },
        "bench_groups": groups
    });

    let out_path = baseline_path(name);
    fs::create_dir_all(baseline_dir()).unwrap_or_else(|e| {
        eprintln!("failed to create baseline dir: {e}");
        process::exit(1);
    });
    let text = serde_json::to_string_pretty(&baseline).unwrap_or_else(|e| {
        eprintln!("failed to serialise baseline: {e}");
        process::exit(1);
    });
    fs::write(&out_path, text + "\n").unwrap_or_else(|e| {
        eprintln!("failed to write {}: {e}", out_path.display());
        process::exit(1);
    });
    println!(
        "xtask bench --capture-baseline {name}: wrote {}",
        out_path.display()
    );
}

// ── Check against ─────────────────────────────────────────────────────────────

fn check_against(name: &str) {
    let bl_path = baseline_path(name);
    let bl_text = fs::read_to_string(&bl_path).unwrap_or_else(|e| {
        eprintln!(
            "xtask bench --check-against: cannot read {}: {e}",
            bl_path.display()
        );
        process::exit(1);
    });
    let baseline: Value = serde_json::from_str(&bl_text).unwrap_or_else(|e| {
        eprintln!(
            "xtask bench --check-against: {}: invalid JSON: {e}",
            bl_path.display()
        );
        process::exit(1);
    });
    let bl_groups = baseline
        .get("bench_groups")
        .and_then(Value::as_object)
        .unwrap_or_else(|| {
            eprintln!("xtask bench --check-against: 'bench_groups' missing or not an object");
            process::exit(1);
        });

    println!("xtask bench --check-against {name}: running criterion …");
    clear_criterion_cache();
    let status = Command::new("cargo")
        .args([
            "bench",
            "--manifest-path",
            "Cargo.toml",
            "-p",
            "tinyquant-bench",
            "--bench",
            "batch_parallel",
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to run cargo bench: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }

    let current_groups = collect_criterion_results();

    // Compare.
    let mut failed = false;
    println!("\n{:<50} {:>14} {:>14} {:>10}", "Group", "Baseline(ns)", "Current(ns)", "vs budget");
    println!("{}", "-".repeat(92));

    for (group_name, bl_entry) in bl_groups {
        let bl_median = bl_entry
            .get("median_ns")
            .and_then(Value::as_f64)
            .unwrap_or(f64::MAX);
        let budget_pct = bl_entry
            .get("budget_pct")
            .and_then(Value::as_f64)
            .unwrap_or(115.0);
        let budget_ns = bl_median * budget_pct / 100.0;

        if let Some(cur_entry) = current_groups.get(group_name) {
            let cur_median = cur_entry
                .get("median_ns")
                .and_then(Value::as_f64)
                .unwrap_or(f64::MAX);
            let ok = cur_median <= budget_ns;
            let marker = if ok { "✓" } else { "✗ REGRESSION" };
            println!("{group_name:<50} {bl_median:>14.0} {cur_median:>14.0} {marker:>10}");
            if !ok {
                failed = true;
            }
        } else {
            println!("{group_name:<50} {bl_median:>14.0} {:>14} {:>10}", "(missing)", "?");
        }
    }

    if failed {
        eprintln!("\nxtask bench --check-against {name}: BUDGET EXCEEDED — see table above");
        process::exit(1);
    } else {
        println!("\nxtask bench --check-against {name}: all groups within budget ✓");
    }
}

// ── Criterion result collection ───────────────────────────────────────────────

/// Walk `target/criterion/` and collect `new/estimates.json` files.
///
/// Returns a map `group_name → {"median_ns": <f64>, "budget_pct": 115}`.
fn collect_criterion_results() -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    // NOTE: this walks ALL of target/criterion/, including results from prior
    // runs with different bench-group names. If group names change between
    // runs, stale directories will be included. For reliable comparisons,
    // clear target/criterion/ before running --capture-baseline or
    // --check-against.
    let crit_dir = criterion_dir();
    if !crit_dir.exists() {
        return out;
    }
    collect_recursive(&crit_dir, &crit_dir, &mut out);
    out
}

fn collect_recursive(
    root: &Path,
    dir: &Path,
    out: &mut serde_json::Map<String, Value>,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(root, &path, out);
        } else if path.file_name() == Some(std::ffi::OsStr::new("estimates.json"))
            && path.parent().and_then(Path::file_name)
                == Some(std::ffi::OsStr::new("new"))
        {
            // Group name is the relative path minus `/new/estimates.json`.
            let group_path = path
                .parent()   // "new/"
                .and_then(Path::parent) // bench group dir
                .unwrap_or(root);
            let group_name = group_path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    let median_ns = v
                        .get("median")
                        .and_then(|m| m.get("point_estimate"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    out.insert(
                        group_name,
                        serde_json::json!({
                            "median_ns": median_ns,
                            // budget_pct: allowable regression budget as a percentage.
                            // To change this per-group, re-capture the baseline after
                            // editing this value; --check-against reads it from the
                            // committed baseline JSON.
                            "budget_pct": 115
                        }),
                    );
                }
            }
        }
    }
}

// ── System info helpers ───────────────────────────────────────────────────────

fn git_short_sha() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_owned(), |s| s.trim().to_owned())
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_owned(), |s| s.trim().to_owned())
}

fn os_arch() -> (String, String) {
    let os = std::env::consts::OS.to_owned();
    let arch = std::env::consts::ARCH.to_owned();
    (os, arch)
}

fn now_utc() -> String {
    // Use ISO-8601 without chrono dep.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

/// Format a UNIX timestamp as `YYYY-MM-DDTHH:MM:SSZ` without chrono.
fn format_utc(secs: u64) -> String {
    // Days from 1970-01-01 to start of each month (non-leap, then leap).
    const DAYS_PER_MONTH: [[u16; 12]; 2] = [
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    ];
    let ss = secs % 60;
    let mm = (secs / 60) % 60;
    let hh = (secs / 3600) % 24;
    // Days since epoch: secs / 86400 is at most ~50k for year 2100 — fits u32.
    #[allow(clippy::cast_possible_truncation)]
    let mut days = (secs / 86400) as u32;

    let mut year = 1970u32;
    loop {
        let leap = is_leap(year);
        let days_in_year: u32 = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = usize::from(is_leap(year));
    let mut month = 0_usize;
    for m in 0..12 {
        let dim = u32::from(DAYS_PER_MONTH[leap][m]);
        if days < dim {
            month = m;
            break;
        }
        days -= dim;
    }
    let day = days + 1;
    let month = month + 1;
    format!("{year:04}-{month:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::format_utc;

    #[test]
    fn epoch_formats_correctly() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn known_timestamp() {
        // 2025-04-11 00:00:00 UTC = 1 744 329 600 seconds from epoch
        assert_eq!(format_utc(1_744_329_600), "2025-04-11T00:00:00Z");
    }

    #[test]
    fn y2k_leap_day() {
        // 2000-02-29 00:00:00 UTC = 951782400 seconds from epoch
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn year_end_boundary() {
        // 2023-12-31 00:00:00 UTC = 1703980800 seconds from epoch
        assert_eq!(format_utc(1_703_980_800), "2023-12-31T00:00:00Z");
    }

    #[test]
    fn non_leap_century_year() {
        // 2100-02-28 00:00:00 UTC = 4107456000 seconds from epoch
        assert_eq!(format_utc(4_107_456_000), "2100-02-28T00:00:00Z");
    }
}
