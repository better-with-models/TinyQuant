//! Xtask `check-matrix-sync` subcommand (Phase 22.D).
//!
//! Asserts that the CLI smoke-test matrix declared in
//! `docs/plans/rust/phase-22-pyo3-cabi-release.md` §CLI smoke test matrix
//! matches the `build` job's `strategy.matrix.include` list in
//! `.github/workflows/rust-release.yml`.
//!
//! The design doc lists every supported target triple twice — once in the
//! human-readable table, once inside the YAML matrix. Drift between the
//! two is a recurring Phase 14 Lesson L3 failure mode, so this subcommand
//! grep-diffs the two lists and exits non-zero on mismatch.
//!
//! The comparison is deliberately **forgiving about ordering** — both
//! sources list triples in a slightly different order for readability —
//! and **strict about membership**: every triple that appears in one
//! list must appear in the other.

use std::{fs, path::Path, process};

/// Entry point.
pub fn run() {
    let repo_root = repo_root();
    let plan_path = repo_root.join("docs/plans/rust/phase-22-pyo3-cabi-release.md");
    let yaml_path = repo_root.join(".github/workflows/rust-release.yml");

    let plan = read_or_die(&plan_path);
    let yaml = read_or_die(&yaml_path);

    let plan_triples = extract_plan_triples(&plan);
    let yaml_triples = extract_yaml_triples(&yaml);

    if plan_triples.is_empty() {
        eprintln!(
            "check-matrix-sync: no triples parsed from {} — did the §CLI smoke test matrix section move?",
            plan_path.display()
        );
        process::exit(1);
    }
    if yaml_triples.is_empty() {
        eprintln!(
            "check-matrix-sync: no triples parsed from {} — did the `build` matrix disappear?",
            yaml_path.display()
        );
        process::exit(1);
    }

    let only_in_plan: Vec<&String> = plan_triples
        .iter()
        .filter(|t| !yaml_triples.contains(t))
        .collect();
    let only_in_yaml: Vec<&String> = yaml_triples
        .iter()
        .filter(|t| !plan_triples.contains(t))
        .collect();

    if only_in_plan.is_empty() && only_in_yaml.is_empty() {
        println!(
            "check-matrix-sync: plan and release.yml agree on {} targets ✓",
            plan_triples.len()
        );
        return;
    }

    eprintln!("check-matrix-sync: drift detected between plan and release.yml");
    if !only_in_plan.is_empty() {
        eprintln!("  present in plan but missing from release.yml:");
        for t in &only_in_plan {
            eprintln!("    - {t}");
        }
    }
    if !only_in_yaml.is_empty() {
        eprintln!("  present in release.yml but missing from plan:");
        for t in &only_in_yaml {
            eprintln!("    - {t}");
        }
    }
    process::exit(1);
}

/// Targets appear in the plan's `| x86_64-unknown-linux-gnu | ubuntu-… |` rows
/// inside the §CLI smoke test matrix section. We grep for any markdown cell
/// whose content looks like a Rust target triple.
fn extract_plan_triples(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        if line.contains("CLI smoke test matrix") {
            in_section = true;
            continue;
        }
        if in_section {
            // Terminate the search when we hit the next top-level heading.
            if line.starts_with("###") && !line.contains("CLI smoke test matrix") {
                break;
            }
            if let Some(cell) = line.strip_prefix("| `") {
                if let Some(end) = cell.find("` |") {
                    let triple = &cell[..end];
                    if looks_like_triple(triple) {
                        out.push(triple.to_owned());
                    }
                }
            }
        }
    }
    out
}

/// Parse the `target:` entries inside the `build:` matrix of release.yml.
/// We look for lines of the form `- { target: <triple>, os: …, cross: … }`.
fn extract_yaml_triples(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("- { target:") else {
            continue;
        };
        let rest = rest.trim_start();
        let triple = rest
            .split(',')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .trim_end_matches(',');
        if looks_like_triple(triple) {
            out.push(triple.to_owned());
        }
    }
    out
}

/// Heuristic triple check: `<arch>-<vendor>-<os>[-<env>]` with at least 3 dashes.
fn looks_like_triple(s: &str) -> bool {
    let dash_count = s.matches('-').count();
    dash_count >= 2
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn read_or_die(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("check-matrix-sync: cannot read {}: {e}", path.display());
        process::exit(1);
    })
}

fn repo_root() -> std::path::PathBuf {
    // xtask runs from the `rust/` directory.
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("check-matrix-sync: cannot read cwd: {e}");
        process::exit(1);
    });
    cwd.parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triple_heuristic_recognises_tier1_targets() {
        for t in [
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-musl",
            "x86_64-unknown-freebsd",
        ] {
            assert!(looks_like_triple(t), "should recognise {t}");
        }
    }

    #[test]
    fn triple_heuristic_rejects_common_non_triples() {
        for t in ["Runner", "ubuntu-22.04", "same", "7zip!"] {
            assert!(!looks_like_triple(t), "should reject {t}");
        }
    }

    #[test]
    fn plan_extractor_picks_up_table_rows() {
        let sample = r"#### CLI smoke test matrix

| Target triple | Runner |
|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` |
| `aarch64-apple-darwin` | `macos-14` |

### Next section
";
        let triples = extract_plan_triples(sample);
        assert_eq!(
            triples,
            vec![
                "x86_64-unknown-linux-gnu".to_owned(),
                "aarch64-apple-darwin".to_owned(),
            ]
        );
    }

    #[test]
    fn yaml_extractor_picks_up_matrix_include() {
        let sample = r"
        include:
          - { target: x86_64-unknown-linux-gnu, os: ubuntu-22.04, cross: false }
          - { target: aarch64-apple-darwin,    os: macos-14,     cross: false }
";
        let triples = extract_yaml_triples(sample);
        assert_eq!(
            triples,
            vec![
                "x86_64-unknown-linux-gnu".to_owned(),
                "aarch64-apple-darwin".to_owned(),
            ]
        );
    }
}
