//! Xtask publish-guard drift check (Phase 22.D code-quality follow-up M3).
//!
//! The release workflow `.github/workflows/rust-release.yml` carries four
//! `publish-*` jobs (`publish-crates`, `publish-pypi`, `publish-container`,
//! `publish-release`) whose top-level `if:` expression must stay byte-
//! identical. If they drift — e.g. one job adds a new pre-release guard
//! but the others are missed — the asymmetry can cause a buggy tag to
//! publish against `crates.io` or `PyPI` while skipping the GitHub release,
//! or vice versa.
//!
//! GitHub Actions does not reliably expand YAML anchors across jobs, so
//! the guard is duplicated verbatim. This module parses the workflow,
//! extracts the `if:` block for every job whose name starts with
//! `publish-`, normalises whitespace, and asserts all extracted strings
//! are equal. It is wired into the existing `cargo xtask
//! check-matrix-sync` entry point so a single invocation enforces both
//! the CLI smoke-matrix drift check and the publish-guard drift check.

use std::{fs, path::Path, process};

/// Run the publish-guard drift check against the real workflow file in
/// the repository root.
///
/// On mismatch, prints every variant with its job name and `exit 1`s.
pub fn run() {
    let repo_root = repo_root();
    let yaml_path = repo_root.join(".github/workflows/rust-release.yml");
    let yaml = fs::read_to_string(&yaml_path).unwrap_or_else(|e| {
        eprintln!(
            "check-publish-guards: cannot read {}: {e}",
            yaml_path.display()
        );
        process::exit(1);
    });

    match check(&yaml) {
        Ok(count) => {
            println!("check-publish-guards: {count} publish-* jobs share the same guard \u{2713}");
        }
        Err(report) => {
            eprintln!("{report}");
            process::exit(1);
        }
    }
}

/// Pure check over an in-memory YAML string.
///
/// Returns `Ok(n)` with the number of agreeing publish jobs on success,
/// or `Err(report)` with a human-readable multiline drift report.
///
/// Visible for unit testing.
pub(crate) fn check(yaml: &str) -> Result<usize, String> {
    let guards = extract_publish_guards(yaml);

    if guards.is_empty() {
        return Err(
            "check-publish-guards: no publish-* jobs found in workflow — did the file move?"
                .to_owned(),
        );
    }

    // Every publish job must agree with the first.
    let (first_name, first_guard) = &guards[0];
    let mut mismatches = Vec::new();
    for (name, guard) in guards.iter().skip(1) {
        if guard != first_guard {
            mismatches.push((name.as_str(), guard.as_str()));
        }
    }

    if mismatches.is_empty() {
        return Ok(guards.len());
    }

    let mut report = String::from(
        "check-publish-guards: publish-* job guards drifted — all four must be byte-identical:\n",
    );
    report.push_str(&format!("  {first_name} (reference):\n    {first_guard}\n"));
    for (name, guard) in &mismatches {
        report.push_str(&format!("  {name} (differs):\n    {guard}\n"));
    }
    Err(report)
}

/// Walk the workflow and extract `(job_name, normalised_guard)` for every
/// top-level job whose name starts with `publish-`.
///
/// The extractor is intentionally string-level — avoiding a YAML parser
/// dep to keep xtask lean (lesson: Phase 21 `matrix_sync.rs`) — and it
/// assumes the workflow's formatting conventions (2-space indent for
/// job keys, `if: >-` folded scalar block for the guard). If the
/// workflow ever switches to a different `if:` style we either teach
/// this parser the new style or fall back to a real YAML crate.
fn extract_publish_guards(yaml: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = yaml.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // Top-level jobs are indented exactly two spaces under `jobs:`.
        // Match `  publish-xxx:` at column 2 (no deeper nesting, no
        // comment-only matches).
        if let Some(name) = top_level_job_name(line) {
            if name.starts_with("publish-") {
                if let Some(guard) = extract_if_block(&lines, i) {
                    out.push((name, guard));
                }
            }
        }
        i += 1;
    }

    out
}

/// Return the job name if `line` declares a top-level job
/// (`  <name>:` with exactly 2 leading spaces, no deeper indent, and
/// the colon terminates the name).
fn top_level_job_name(line: &str) -> Option<String> {
    // Require exactly two leading spaces.
    if !line.starts_with("  ") || line.starts_with("   ") {
        return None;
    }
    let trimmed = &line[2..];
    // Must end in `:` with no trailing value.
    let without_comment = trimmed.split('#').next().unwrap_or(trimmed).trim_end();
    let name = without_comment.strip_suffix(':')?;
    // Reject things like `  - foo:` or empty names.
    if name.is_empty() || !name.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Some(name.to_owned())
    } else {
        None
    }
}

/// From the `job:` header at `lines[job_idx]`, walk forward looking for
/// the `if:` key inside that job's body and return its textual content
/// with whitespace normalised to single spaces and the leading
/// block-scalar indicator stripped.
///
/// Stops at the next top-level job so we can never accidentally slurp
/// the following job's `if:`.
fn extract_if_block(lines: &[&str], job_idx: usize) -> Option<String> {
    let mut i = job_idx + 1;
    while i < lines.len() {
        let line = lines[i];
        // End of this job: next top-level job starts.
        if top_level_job_name(line).is_some() {
            return None;
        }
        // Match `    if:` — indented four spaces inside a top-level job.
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("if:") {
            return Some(read_if_body(lines, i, rest.trim()));
        }
        i += 1;
    }
    None
}

/// Given the line where `if:` appears (`rest` is everything after `if:`
/// on that same line, already trimmed), read the full expression —
/// including folded-scalar continuation lines — and return it
/// whitespace-normalised.
fn read_if_body(lines: &[&str], if_idx: usize, rest_of_line: &str) -> String {
    // The guard style used in rust-release.yml is `if: >-` (folded
    // block scalar with strip-chomping). That means `rest_of_line`
    // starts with `>-` (or `>`, `|`, `|-`, etc.) and the actual
    // expression is on the following indented lines.
    //
    // For a non-block `if: some-expr` (inline), the inline text IS the
    // body. Handle both shapes.

    let first_char = rest_of_line.chars().next();
    let mut parts: Vec<String> = Vec::new();
    let mut start_next_line = if matches!(first_char, Some('>' | '|')) {
        // Block scalar. Ignore the indicator line itself.
        if_idx + 1
    } else if rest_of_line.is_empty() {
        if_idx + 1
    } else {
        parts.push(rest_of_line.to_owned());
        lines.len() // signal: no continuation
    };

    // Determine the indent of the first continuation line to know when
    // the block ends.
    let mut block_indent: Option<usize> = None;
    while start_next_line < lines.len() {
        let line = lines[start_next_line];
        let indent = line.chars().take_while(|c| c == &' ').count();
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            start_next_line += 1;
            continue;
        }
        // End of block: dedent back to the enclosing mapping level
        // (the `if:` line itself is indented 4 spaces under a job, so
        // the block body is at >= 6 spaces).
        if let Some(expected) = block_indent {
            if indent < expected {
                break;
            }
        } else {
            block_indent = Some(indent);
        }
        parts.push(trimmed.to_owned());
        start_next_line += 1;
    }

    // Normalise: collapse every run of whitespace into a single space
    // so cosmetic reflows don't cause false drift.
    let joined = parts.join(" ");
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn repo_root() -> std::path::PathBuf {
    // xtask runs from the `rust/` directory.
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("check-publish-guards: cannot read cwd: {e}");
        process::exit(1);
    });
    cwd.parent().map(Path::to_path_buf).unwrap_or(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOUR_JOBS_AGREE: &str = r"---
name: rust-release
on: {}
jobs:
  build:
    name: Build
    runs-on: ubuntu-22.04
    steps:
      - run: echo ok
  release-gate:
    name: Release gate
    runs-on: ubuntu-22.04
    outputs:
      should_publish: ${{ steps.evaluate.outputs.should_publish }}
    steps:
      - id: evaluate
        run: echo should_publish=true
  publish-crates:
    name: Publish crates.io
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
  publish-pypi:
    name: Publish PyPI
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
  publish-container:
    name: Publish GHCR image
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
  publish-release:
    name: GitHub release
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
";

    const ONE_JOB_DIFFERS: &str = r"---
jobs:
  publish-crates:
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
  publish-pypi:
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && !contains(github.ref_name, '-alpha')
      && inputs.dry_run != true
    steps:
      - run: echo publish
  publish-container:
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
  publish-release:
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
";

    #[test]
    fn all_four_agree_passes() {
        let result = check(FOUR_JOBS_AGREE);
        match result {
            Ok(n) => assert_eq!(n, 4, "expected 4 publish jobs to agree, got {n}"),
            Err(report) => panic!("expected Ok, got Err:\n{report}"),
        }
    }

    #[test]
    fn one_job_differs_fails() {
        let result = check(ONE_JOB_DIFFERS);
        match result {
            Ok(n) => panic!("expected drift to be detected, got Ok({n})"),
            Err(report) => {
                assert!(
                    report.contains("publish-pypi"),
                    "report should name the drifting job, got:\n{report}"
                );
                assert!(
                    report.contains("-alpha"),
                    "report should include the drifting guard text, got:\n{report}"
                );
            }
        }
    }

    #[test]
    fn extract_finds_exactly_four_publish_jobs() {
        let guards = extract_publish_guards(FOUR_JOBS_AGREE);
        let names: Vec<&str> = guards.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "publish-crates",
                "publish-pypi",
                "publish-container",
                "publish-release",
            ]
        );
    }

    #[test]
    fn extract_skips_non_publish_jobs() {
        let guards = extract_publish_guards(FOUR_JOBS_AGREE);
        assert!(
            guards.iter().all(|(n, _)| n.starts_with("publish-")),
            "non-publish jobs leaked into the extraction"
        );
    }

    #[test]
    fn whitespace_differences_do_not_cause_false_drift() {
        // Same expression, different internal whitespace — should
        // still be treated as equal by the normaliser.
        let yaml = r"---
jobs:
  publish-a:
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
  publish-b:
    if: >-
      needs.release-gate.outputs.should_publish   ==    'true'
      && inputs.dry_run    !=   true
";
        let result = check(yaml);
        assert!(result.is_ok(), "cosmetic whitespace should be normalised");
    }
}
