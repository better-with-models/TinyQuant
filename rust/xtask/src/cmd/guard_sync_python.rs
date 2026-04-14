//! Phase 24.3: publish-guard drift check for `python-fatwheel.yml`.
//!
//! The Python fat-wheel release workflow has a single `publish` job
//! (unlike `rust-release.yml`, which has four). Rather than an
//! all-four-agree equality check, this module asserts the single
//! `publish` job's `if:` expression matches the contract documented
//! in `docs/plans/rust/phase-24-python-fat-wheel-official.md`:
//!
//!     needs.release-gate.outputs.should_publish == 'true'
//!     && inputs.dry_run != true
//!
//! The contract is deliberately byte-identical to the four `publish-*`
//! guards in `rust-release.yml` so both release workflows share a
//! single uniform publish-gate shape. If `python-fatwheel.yml` drifts
//! — e.g. a contributor drops the `inputs.dry_run != true` clause —
//! this check fires in CI before the workflow can accidentally reach
//! PyPI on a rehearsal run.
//!
//! The module reuses `guard_sync`'s string-level extractor
//! (`extract_if_block`, `top_level_job_name`) by re-parsing from the
//! same line conventions. Keeping it a sibling module (rather than a
//! generic parameterised check) mirrors the Phase 22.D Option A
//! architecture — one guard, one module, one failure message.
//!
//! Wired into `cargo xtask check-publish-guards` so a single verb
//! enforces both the four-way equality on `rust-release.yml` and the
//! contract equality on `python-fatwheel.yml`.

use std::{fs, path::Path, process};

/// Canonical contract expression — whitespace-normalised identically
/// to how `read_if_body` would normalise the actual workflow text.
/// Duplicated verbatim (not imported) because drifting this constant
/// is precisely the failure mode the check exists to catch.
const CONTRACT: &str =
    "needs.release-gate.outputs.should_publish == 'true' && inputs.dry_run != true";

/// Run the python-fatwheel publish-guard check against the real
/// workflow in the repository root.
pub fn run() {
    let repo_root = repo_root();
    let yaml_path = repo_root.join(".github/workflows/python-fatwheel.yml");
    let yaml = fs::read_to_string(&yaml_path).unwrap_or_else(|e| {
        eprintln!(
            "check-publish-guards[python]: cannot read {}: {e}",
            yaml_path.display()
        );
        process::exit(1);
    });

    match check(&yaml) {
        Ok(()) => {
            println!(
                "check-publish-guards[python]: publish job guard matches contract \u{2713}"
            );
        }
        Err(report) => {
            eprintln!("{report}");
            process::exit(1);
        }
    }
}

/// Pure check over an in-memory YAML string.
///
/// Returns `Ok(())` if the `publish` job's `if:` block matches the
/// canonical contract string verbatim (after whitespace normalisation).
pub(crate) fn check(yaml: &str) -> Result<(), String> {
    let guard = extract_publish_guard(yaml).ok_or_else(|| {
        "check-publish-guards[python]: no `publish` job with an `if:` block \
         found — did the workflow file move or rename the job?"
            .to_owned()
    })?;

    if guard == CONTRACT {
        Ok(())
    } else {
        Err(format!(
            "check-publish-guards[python]: `publish` job guard drifted from contract.\n\
             expected (contract):\n    {CONTRACT}\n\
             actual (workflow):\n    {guard}\n\
             fix: restore the two-clause guard or update the contract in \
             rust/xtask/src/cmd/guard_sync_python.rs with justification."
        ))
    }
}

/// Walk `python-fatwheel.yml` looking for the single top-level
/// `publish:` job and return its normalised `if:` expression.
fn extract_publish_guard(yaml: &str) -> Option<String> {
    let lines: Vec<&str> = yaml.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some(name) = top_level_job_name(lines[i]) {
            if name == "publish" {
                return extract_if_block(&lines, i);
            }
        }
        i += 1;
    }
    None
}

/// Mirror of `guard_sync::top_level_job_name`.
fn top_level_job_name(line: &str) -> Option<String> {
    if !line.starts_with("  ") || line.starts_with("   ") {
        return None;
    }
    let trimmed = &line[2..];
    let without_comment = trimmed.split('#').next().unwrap_or(trimmed).trim_end();
    let name = without_comment.strip_suffix(':')?;
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

/// Mirror of `guard_sync::extract_if_block`.
fn extract_if_block(lines: &[&str], job_idx: usize) -> Option<String> {
    let mut i = job_idx + 1;
    while i < lines.len() {
        let line = lines[i];
        if top_level_job_name(line).is_some() {
            return None;
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("if:") {
            return Some(read_if_body(lines, i, rest.trim()));
        }
        i += 1;
    }
    None
}

/// Mirror of `guard_sync::read_if_body`.
fn read_if_body(lines: &[&str], if_idx: usize, rest_of_line: &str) -> String {
    let first_char = rest_of_line.chars().next();
    let mut parts: Vec<String> = Vec::new();
    let mut start_next_line = if matches!(first_char, Some('>' | '|')) {
        if_idx + 1
    } else if rest_of_line.is_empty() {
        if_idx + 1
    } else {
        parts.push(rest_of_line.to_owned());
        lines.len()
    };

    let mut block_indent: Option<usize> = None;
    while start_next_line < lines.len() {
        let line = lines[start_next_line];
        let indent = line.chars().take_while(|c| c == &' ').count();
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            start_next_line += 1;
            continue;
        }
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

    let joined = parts.join(" ");
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn repo_root() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("check-publish-guards[python]: cannot read cwd: {e}");
        process::exit(1);
    });
    cwd.parent().map(Path::to_path_buf).unwrap_or(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATCHES_CONTRACT: &str = r"---
name: python-fatwheel
on: {}
jobs:
  release-gate:
    name: Release gate
    runs-on: ubuntu-22.04
    outputs:
      should_publish: ${{ steps.evaluate.outputs.should_publish }}
    steps:
      - id: evaluate
        run: echo should_publish=true
  assemble:
    runs-on: ubuntu-22.04
    steps:
      - run: echo assemble
  install-test:
    runs-on: ubuntu-22.04
    steps:
      - run: echo test
  publish:
    needs: [install-test, release-gate]
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      && inputs.dry_run != true
    steps:
      - run: echo publish
";

    const DRIFT_MISSING_DRY_RUN_CLAUSE: &str = r"---
jobs:
  publish:
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
    steps:
      - run: echo publish
";

    const DRIFT_WRONG_OPERATOR: &str = r"---
jobs:
  publish:
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish == 'true'
      || inputs.dry_run != true
    steps:
      - run: echo publish
";

    const NO_PUBLISH_JOB: &str = r"---
jobs:
  assemble:
    runs-on: ubuntu-22.04
    steps:
      - run: echo assemble
";

    #[test]
    fn contract_expression_passes() {
        check(MATCHES_CONTRACT).expect("contract-matching workflow should pass");
    }

    #[test]
    fn missing_dry_run_clause_fails() {
        let err = check(DRIFT_MISSING_DRY_RUN_CLAUSE).expect_err("drift must fail");
        assert!(
            err.contains("dry_run"),
            "report should mention the missing clause, got: {err}"
        );
    }

    #[test]
    fn wrong_boolean_operator_fails() {
        let err = check(DRIFT_WRONG_OPERATOR).expect_err("|| vs && drift must fail");
        assert!(
            err.contains("drifted from contract"),
            "report should flag drift, got: {err}"
        );
    }

    #[test]
    fn missing_publish_job_fails() {
        let err = check(NO_PUBLISH_JOB).expect_err("absent publish job must fail");
        assert!(
            err.contains("publish"),
            "report should mention the missing job, got: {err}"
        );
    }

    #[test]
    fn whitespace_reflow_does_not_cause_false_drift() {
        let reflowed = r"---
jobs:
  publish:
    runs-on: ubuntu-22.04
    if: >-
      needs.release-gate.outputs.should_publish   ==   'true'
      &&    inputs.dry_run    !=    true
    steps:
      - run: echo publish
";
        check(reflowed).expect("cosmetic whitespace must normalise to the contract");
    }
}
