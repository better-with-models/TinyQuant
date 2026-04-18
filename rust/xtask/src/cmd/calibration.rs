//! Xtask calibration subcommand.
//!
//! Calibration tests (`Codebook::train` on 1k×768 vectors) are too slow for
//! regular CI in debug mode.  Results are captured locally with `--release`
//! and committed as `baselines/calibration-results.json`; CI validates only
//! the committed artifact (schema check, no codec execution).
//!
//! # Subcommands
//!
//! | Command | Description |
//! |---------|-------------|
//! | `--validate` | Assert `baselines/calibration-results.json` matches schema. |
//! | `--capture-results` | Run ignored `pr_speed` tests with `--release`, parse output, write JSON. |
//!
//! # Workflow
//!
//! ```text
//! # After a codec change that may affect quality:
//! cargo xtask calibration --capture-results
//! git add rust/crates/tinyquant-bench/baselines/calibration-results.json
//! git commit -m "chore(calibration): update results"
//! ```

use std::{
    fs,
    path::PathBuf,
    process::{self, Command},
};

use serde_json::Value;

/// Entry point — parse `args` and dispatch.
pub fn run(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("--validate") => validate(),
        Some("--capture-results") => capture_results(),
        _ => {
            eprintln!("usage: cargo xtask calibration <--validate|--capture-results>");
            process::exit(1);
        }
    }
}

// ── Paths ────────────────────────────────────────────────────────────────────

fn baseline_dir() -> PathBuf {
    PathBuf::from("crates/tinyquant-bench/baselines")
}

fn results_path() -> PathBuf {
    baseline_dir().join("calibration-results.json")
}

fn schema_path() -> PathBuf {
    baseline_dir().join("calibration-schema.json")
}

// ── Validate ─────────────────────────────────────────────────────────────────

fn validate() {
    let res_path = results_path();
    let res_text = fs::read_to_string(&res_path).unwrap_or_else(|e| {
        eprintln!(
            "xtask calibration --validate: cannot read {}: {e}\n\
             Capture results locally: cargo xtask calibration --capture-results",
            res_path.display()
        );
        process::exit(1);
    });
    let results: Value = serde_json::from_str(&res_text).unwrap_or_else(|e| {
        eprintln!(
            "xtask calibration --validate: {}: invalid JSON: {e}",
            res_path.display()
        );
        process::exit(1);
    });

    let schema_text = fs::read_to_string(schema_path()).unwrap_or_else(|e| {
        eprintln!("xtask calibration --validate: cannot read calibration-schema.json: {e}");
        process::exit(1);
    });
    let schema: Value = serde_json::from_str(&schema_text).unwrap_or_else(|e| {
        eprintln!("xtask calibration --validate: calibration-schema.json is invalid JSON: {e}");
        process::exit(1);
    });

    // schema_version check.
    let sv = results
        .get("schema_version")
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    let expected_sv = schema
        .get("properties")
        .and_then(|p| p.get("schema_version"))
        .and_then(|s| s.get("const"))
        .and_then(Value::as_i64)
        .unwrap_or(1);
    if sv != expected_sv {
        eprintln!(
            "xtask calibration --validate: schema_version={sv} != expected {expected_sv}"
        );
        process::exit(1);
    }

    // Required top-level fields.
    for field in ["captured_at", "git_commit", "host", "corpus", "seed", "results"] {
        if results.get(field).is_none() {
            eprintln!(
                "xtask calibration --validate: missing required field '{field}'"
            );
            process::exit(1);
        }
    }

    // Required result keys.
    let required_keys = [
        "bw4_residual",
        "bw4_no_residual",
        "bw2_residual",
        "bw8_residual",
        "bw2_no_residual",
    ];
    let result_obj = results
        .get("results")
        .and_then(Value::as_object)
        .unwrap_or_else(|| {
            eprintln!("xtask calibration --validate: 'results' is not an object");
            process::exit(1);
        });
    for key in required_keys {
        let entry = result_obj.get(key).unwrap_or_else(|| {
            eprintln!("xtask calibration --validate: missing result entry '{key}'");
            process::exit(1);
        });
        for metric in ["rho", "recall_at_10", "ratio"] {
            if entry.get(metric).and_then(Value::as_f64).is_none() {
                eprintln!(
                    "xtask calibration --validate: '{key}.{metric}' missing or not a number"
                );
                process::exit(1);
            }
        }
    }

    println!(
        "xtask calibration --validate: {} is valid ({} result entries) ✓",
        res_path.display(),
        result_obj.len()
    );
}

// ── Capture results ───────────────────────────────────────────────────────────

/// Run the ignored `pr_speed` calibration tests with `--release`, parse the
/// `CALIBRATION_RESULT` lines they emit, and write `calibration-results.json`.
fn capture_results() {
    println!("xtask calibration --capture-results: running pr_speed tests (--release) …");

    let output = Command::new("cargo")
        .args([
            "test",
            "--release",
            "-p",
            "tinyquant-bench",
            "--",
            "--ignored",
            "pr_speed",
            "--nocapture",
        ])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to invoke cargo test: {e}");
            process::exit(1);
        });

    if !output.status.success() {
        eprintln!(
            "xtask calibration --capture-results: tests FAILED — thresholds not met.\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        process::exit(output.status.code().unwrap_or(1));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries = parse_calibration_results(&stdout);
    if entries.is_empty() {
        eprintln!(
            "xtask calibration --capture-results: no CALIBRATION_RESULT lines found in output.\n\
             Ensure run_gate() prints them and that --nocapture is working."
        );
        process::exit(1);
    }

    let git_commit = git_short_sha();
    let rustc_ver = rustc_version();
    let (os, arch) = os_arch();

    let mut results_map = serde_json::Map::new();
    for (key, rho, recall, ratio) in &entries {
        results_map.insert(
            key.clone(),
            serde_json::json!({
                "rho":         rho,
                "recall_at_10": recall,
                "ratio":       ratio,
            }),
        );
        println!("  {key}: rho={rho:.4} recall@10={recall:.4} ratio={ratio:.4}");
    }

    let doc = serde_json::json!({
        "schema_version": 1,
        "captured_at":    now_utc(),
        "git_commit":     git_commit,
        "host": {
            "os":        os,
            "arch":      arch,
            "cpu_model": cpu_model(),
            "rustc":     rustc_ver,
        },
        "corpus": "openai_1k_d768",
        "seed":   42,
        "results": results_map,
    });

    let out_path = results_path();
    fs::create_dir_all(baseline_dir()).unwrap_or_else(|e| {
        eprintln!("failed to create baseline dir: {e}");
        process::exit(1);
    });
    let text = serde_json::to_string_pretty(&doc).unwrap_or_else(|e| {
        eprintln!("failed to serialise results: {e}");
        process::exit(1);
    });
    fs::write(&out_path, text + "\n").unwrap_or_else(|e| {
        eprintln!("failed to write {}: {e}", out_path.display());
        process::exit(1);
    });
    println!(
        "xtask calibration --capture-results: wrote {} ({} entries)",
        out_path.display(),
        entries.len()
    );
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Parse `CALIBRATION_RESULT` lines from test stdout.
///
/// Expected format (emitted by `run_gate` in `tests/calibration.rs`):
/// ```text
/// CALIBRATION_RESULT bw=4 residual=true rho=1.000000 recall_at_10=1.000000 ratio=1.600000
/// ```
///
/// Returns `(key, rho, recall_at_10, ratio)` tuples.
fn parse_calibration_results(stdout: &str) -> Vec<(String, f64, f64, f64)> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if !line.starts_with("CALIBRATION_RESULT ") {
            continue;
        }
        let Some(bw) = extract_value(line, "bw=") else { continue };
        let Some(residual) = extract_value(line, "residual=") else { continue };
        let Some(rho) = extract_f64(line, "rho=") else { continue };
        let Some(recall) = extract_f64(line, "recall_at_10=") else { continue };
        let Some(ratio) = extract_f64(line, "ratio=") else { continue };

        let key = match (bw.as_str(), residual.as_str()) {
            ("4", "true")  => "bw4_residual",
            ("4", "false") => "bw4_no_residual",
            ("2", "true")  => "bw2_residual",
            ("2", "false") => "bw2_no_residual",
            ("8", "true")  => "bw8_residual",
            ("8", "false") => "bw8_no_residual",
            _ => {
                eprintln!("warning: unknown bw={bw} residual={residual} in CALIBRATION_RESULT line — skipping");
                continue;
            }
        };
        out.push((key.to_owned(), rho, recall, ratio));
    }
    out
}

/// Extract the string token after `prefix=` (stops at the next space).
fn extract_value(line: &str, prefix: &str) -> Option<String> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    Some(rest[..end].to_owned())
}

/// Extract an `f64` value after `prefix=` (stops at the next space).
fn extract_f64(line: &str, prefix: &str) -> Option<f64> {
    extract_value(line, prefix)?.parse().ok()
}

// ── System info helpers (shared with cmd::bench) ──────────────────────────────

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
    (
        std::env::consts::OS.to_owned(),
        std::env::consts::ARCH.to_owned(),
    )
}

fn cpu_model() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = fs::read_to_string("/proc/cpuinfo") {
            for line in text.lines() {
                if let Some(val) = line
                    .strip_prefix("model name\t: ")
                    .or_else(|| line.strip_prefix("Model name\t: "))
                {
                    return val.trim().to_owned();
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            if let Ok(s) = String::from_utf8(out.stdout) {
                let s = s.trim().to_owned();
                if !s.is_empty() {
                    return s;
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = Command::new("wmic")
            .args(["cpu", "get", "Name", "/value"])
            .output()
        {
            if let Ok(s) = String::from_utf8(out.stdout) {
                for line in s.lines() {
                    if let Some(val) = line.strip_prefix("Name=") {
                        let val = val.trim().to_owned();
                        if !val.is_empty() {
                            return val;
                        }
                    }
                }
            }
        }
    }
    "unknown".to_owned()
}

fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

fn format_utc(secs: u64) -> String {
    const DAYS_PER_MONTH: [[u16; 12]; 2] = [
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    ];
    let ss = secs % 60;
    let mm = (secs / 60) % 60;
    let hh = (secs / 3600) % 24;
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
    use super::{extract_f64, extract_value, parse_calibration_results};

    #[test]
    fn parse_single_result_line() {
        let line = "CALIBRATION_RESULT bw=4 residual=true rho=1.000000 recall_at_10=1.000000 ratio=1.600000";
        let results = parse_calibration_results(line);
        assert_eq!(results.len(), 1);
        let (key, rho, recall, ratio) = &results[0];
        assert_eq!(key, "bw4_residual");
        assert!((rho - 1.0).abs() < 1e-9);
        assert!((recall - 1.0).abs() < 1e-9);
        assert!((ratio - 1.6).abs() < 1e-9);
    }

    #[test]
    fn parse_no_residual_line() {
        let line = "CALIBRATION_RESULT bw=4 residual=false rho=0.957300 recall_at_10=0.791000 ratio=8.000000";
        let results = parse_calibration_results(line);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "bw4_no_residual");
    }

    #[test]
    fn extract_value_stops_at_space() {
        let line = "CALIBRATION_RESULT bw=4 residual=true rho=0.99";
        assert_eq!(extract_value(line, "bw=").as_deref(), Some("4"));
        assert_eq!(extract_value(line, "residual=").as_deref(), Some("true"));
    }

    #[test]
    fn extract_f64_parses_float() {
        let line = "rho=0.957300 recall_at_10=1.000000";
        assert!((extract_f64(line, "rho=").unwrap() - 0.9573).abs() < 1e-6);
    }

    #[test]
    fn ignores_non_calibration_lines() {
        let stdout = "running 5 tests\ntest foo ... ok\nCALIBRATION_RESULT bw=2 residual=true rho=1.0 recall_at_10=1.0 ratio=1.7778\ntest result: ok";
        let results = parse_calibration_results(stdout);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "bw2_residual");
    }
}
