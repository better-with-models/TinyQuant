---
title: GitHub Actions Node.js 24 Upgrade
tags:
  - cd
  - ci
  - github-actions
  - plan
  - node24
date-created: 2026-04-09
status: draft
category: ci-cd
---

# GitHub Actions Node.js 24 Upgrade Implementation Plan

> [!info] For agentic workers
> REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended)
> or `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove every "Node.js 20 actions are deprecated" warning from the
`Release` and `CI` workflows by upgrading or replacing actions that still run on
Node 20, so the release pipeline keeps working after GitHub's 2026-06-02 forced
cutover.

**Architecture:** Three mechanical fixes in two workflow files, plus one
wiki-reconciliation step.

1. Bump `actions/upload-artifact` and `actions/download-artifact` to their
   first Node-24 majors (`@v5`) wherever they appear in `.github/workflows/`.
2. Replace `softprops/action-gh-release@v2` with an inline `gh release create`
   Bash step — upstream has no Node-24 release
   ([softprops/action-gh-release#742](https://github.com/softprops/action-gh-release/issues/742);
   tracking PRs #670 and #774 are still open as of 2026-04-09), so removing the
   third-party dependency is less risky than waiting.
3. Reconcile [[CD-plan/release-workflow|Release Workflow]] so the wiki reflects
   the current workflow file (per AGENTS.md code-doc-drift rule).

**Tech Stack:** GitHub Actions, Bash, the `gh` CLI that is pre-installed on
`ubuntu-latest` runners.

---

## Warning inventory (verified 2026-04-09)

> [!note] Source of truth
> Scanned via `Grep` on `.github/workflows/*.yml`. The user-reported warnings
> cover only the release workflow, but the CI workflow has the same
> deprecation and is fixed in the same plan to avoid a second round-trip.

| File | Line | Action | Current | Fix |
| ---- | ---- | ------ | ------- | --- |
| `.github/workflows/release.yml` | 100 | `actions/upload-artifact` | `@v4` | → `@v5` |
| `.github/workflows/release.yml` | 117 | `actions/download-artifact` | `@v4` | → `@v5` |
| `.github/workflows/release.yml` | 154 | `actions/download-artifact` | `@v4` | → `@v5` |
| `.github/workflows/release.yml` | 189 | `actions/download-artifact` | `@v4` | → `@v5` |
| `.github/workflows/release.yml` | 196 | `softprops/action-gh-release` | `@v2` | → **replace with `gh release create`** |
| `.github/workflows/ci.yml` | 161 | `actions/upload-artifact` | `@v4` | → `@v5` |
| `.github/workflows/ci.yml` | 191 | `actions/upload-artifact` | `@v4` | → `@v5` |

> [!info] Version choice rationale
> Verified via `gh api repos/actions/upload-artifact/tags` and
> `gh api repos/actions/download-artifact/tags`:
> - `actions/upload-artifact@v5` was the first Node-24 major (v6 and v7 exist
>   but add features we do not use).
> - `actions/download-artifact@v5` was the first Node-24 major (v6, v7, v8
>   exist similarly).
>
> We pick the **oldest Node-24 major** for the smallest behavioral delta from
> `@v4`, since our usage (one named single-artifact upload/download) is the
> simplest possible pattern and has no known breaking changes across v4→v5.

---

## File structure

This plan does not create new files. It modifies:

- `.github/workflows/release.yml` — version bumps + one step rewrite
- `.github/workflows/ci.yml` — version bumps only
- `docs/CD-plan/release-workflow.md` — wiki reconciliation
- `docs/log.md` — append-only operational log entry
- `docs/index.md` — add entry for this plan page

No new source files, tests, or build artifacts are produced. This is a
workflow maintenance change only.

---

## Task 1: Create a working branch

**Files:** none — git operations only.

- [ ] **Step 1: Confirm clean working tree**

Run: `git status`
Expected: `nothing to commit, working tree clean`

- [ ] **Step 2: Create branch**

Run: `git switch -c chore/gh-actions-node24`
Expected: `Switched to a new branch 'chore/gh-actions-node24'`

---

## Task 2: Bump artifact actions in `release.yml`

**Files:**
- Modify: `.github/workflows/release.yml:100`
- Modify: `.github/workflows/release.yml:117`
- Modify: `.github/workflows/release.yml:154`
- Modify: `.github/workflows/release.yml:189`

- [ ] **Step 1: Bump the upload-artifact step (build job)**

At `.github/workflows/release.yml:100`, change:

```yaml
      - name: Upload dist artifact
        uses: actions/upload-artifact@v4
```

to:

```yaml
      - name: Upload dist artifact
        uses: actions/upload-artifact@v5
```

- [ ] **Step 2: Bump the three download-artifact steps**

At lines 117 (`publish-testpypi`), 154 (`publish-pypi`), and 189
(`github-release`), change each:

```yaml
      - name: Download dist artifact
        uses: actions/download-artifact@v4
```

to:

```yaml
      - name: Download dist artifact
        uses: actions/download-artifact@v5
```

> [!tip] Bulk replacement
> All three download-artifact occurrences are identical. A single
> `Edit` with `replace_all: true` scoped to `release.yml` is safe here because
> there are no non-target occurrences of `actions/download-artifact@v4` in
> that file.

- [ ] **Step 3: Verify no remaining `@v4` for artifact actions**

Use `Grep` with pattern `actions/(upload|download)-artifact@v4` on
`.github/workflows/release.yml`.
Expected: zero matches.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): bump actions/upload-artifact and download-artifact to @v5 (Node 24)"
```

---

## Task 3: Replace `softprops/action-gh-release` with `gh release create`

**Files:**
- Modify: `.github/workflows/release.yml:194-210` (the `Create release` step)

> [!warning] Why replace instead of bump
> `softprops/action-gh-release@v2.6.1` (latest as of 2026-04-09) still declares
> `runs.using: "node20"` in its `action.yml`. The master branch is the same.
> Tracking issue [#742](https://github.com/softprops/action-gh-release/issues/742)
> and PRs #670 / #774 for the Node 24 upgrade are **still open**. With the
> 2026-06-02 deadline ~8 weeks away, replacing the step with the
> pre-installed `gh` CLI removes the blocking dependency and simplifies the
> workflow (one less third-party action to trust).

- [ ] **Step 1: Remove the `softprops/action-gh-release@v2` step**

Delete lines 195–210 of `.github/workflows/release.yml` (the full
`- name: Create release` step and its `uses:` + `with:` block).

- [ ] **Step 2: Replace with a `gh release create` Bash step**

Insert in the same place (after the `Download dist artifact` step in the
`github-release` job):

```yaml
      - name: Create release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          VERSION: ${{ needs.verify-tag.outputs.version }}
          NOTES: ${{ steps.notes.outputs.notes }}
        run: |
          NOTES_FILE=$(mktemp)
          cat > "$NOTES_FILE" <<EOF
          ## Changes

          ${NOTES}

          ## Installation

          \`\`\`bash
          pip install tinyquant-cpu==${VERSION}
          \`\`\`
          EOF

          args=(
            "${GITHUB_REF_NAME}"
            --title "${GITHUB_REF_NAME}"
            --notes-file "$NOTES_FILE"
          )
          if [[ "${GITHUB_REF_NAME}" == *-* ]]; then
            args+=(--prerelease)
          fi
          # Attach built distributions
          args+=(dist/*)

          gh release create "${args[@]}"
```

> [!info] Auth model
> The `github-release` job already declares `permissions: contents: write`
> (line 168). That scope on the default `GITHUB_TOKEN` is exactly what
> `gh release create` needs — no extra secrets required.

> [!info] Prerelease detection
> The original action used `prerelease: ${{ contains(github.ref_name, '-') }}`
> so that tags like `v1.0.0-rc1` are marked as prereleases. The Bash
> `[[ "${GITHUB_REF_NAME}" == *-* ]]` test preserves that behavior.

> [!info] Shell quoting
> The heredoc uses unquoted `EOF` so `${NOTES}` and `${VERSION}` expand. The
> backticks in the installation block are escaped with `\`` to stop the shell
> from treating them as command substitution. `env:` passes values in, which
> avoids GitHub Actions expression injection risks that direct
> `${{ ... }}` interpolation inside `run:` would create.

- [ ] **Step 3: Verify `softprops/action-gh-release` is no longer referenced**

Use `Grep` with pattern `softprops/action-gh-release` on `.github/workflows/`.
Expected: zero matches.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): replace softprops/action-gh-release with gh CLI (Node 24)"
```

---

## Task 4: Bump artifact actions in `ci.yml`

**Files:**
- Modify: `.github/workflows/ci.yml:161`
- Modify: `.github/workflows/ci.yml:191`

- [ ] **Step 1: Bump the coverage upload step**

At `.github/workflows/ci.yml:161`, change `actions/upload-artifact@v4` to
`actions/upload-artifact@v5`.

- [ ] **Step 2: Bump the build-artifact upload step**

At `.github/workflows/ci.yml:191`, change `actions/upload-artifact@v4` to
`actions/upload-artifact@v5`.

- [ ] **Step 3: Verify zero deprecated references remain across all workflows**

Use `Grep` with pattern `@v4` scoped to `.github/workflows/`, then inspect
each hit. Only non-JavaScript-action references (e.g. `pgvector/pgvector:pg17`
is fine, `DavidAnson/markdownlint-cli2-action@v23` is fine at its own major)
and any future non-artifact actions are allowed. No `actions/upload-artifact@v4`
or `actions/download-artifact@v4` should remain.

Expected: zero hits for `(upload|download)-artifact@v4`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: bump actions/upload-artifact to @v5 (Node 24)"
```

---

## Task 5: Local workflow linting

**Files:** none — validation only.

> [!tip] Why no `act` run
> A full `act`-based release simulation would need secrets, environments, and
> a real tag — too heavy for a version-bump change. Syntax linting plus a
> branch push that the CI workflow exercises is sufficient.

- [ ] **Step 1: YAML parse check**

Run:

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"
python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

Expected: both commands exit 0 with no output.

- [ ] **Step 2: Shellcheck the new `gh release create` step**

Extract the `run:` block from the new `Create release` step into a temp file
and run:

```bash
shellcheck -s bash /tmp/create-release.sh
```

Expected: no errors (warnings about `SC2086` on `dist/*` globbing are
acceptable — we intend glob expansion).

If `shellcheck` is not installed, skip this step and rely on the post-push
CI run instead.

- [ ] **Step 3: Optional `actionlint`**

If `actionlint` is available locally:

```bash
actionlint .github/workflows/release.yml .github/workflows/ci.yml
```

Expected: no errors. If not available, skip.

---

## Task 6: Reconcile the wiki

**Files:**
- Modify: `docs/CD-plan/release-workflow.md:231` (the stale
  `softprops/action-gh-release@v2` reference in the embedded job YAML)
- Append: `docs/log.md`
- Modify: `docs/index.md`

> [!warning] Obsidian mode
> Files under `docs/` use Obsidian-flavored markdown with wikilinks, YAML
> frontmatter, and callouts (per `AGENTS.md`). Do not introduce
> markdownlint-only style.

- [ ] **Step 1: Update `release-workflow.md` embedded YAML**

In `docs/CD-plan/release-workflow.md`, the `### github-release` subsection
(around lines 194–246) embeds a snippet of the workflow that still shows the
old `softprops/action-gh-release@v2` step. Replace that snippet to match the
new `gh release create` Bash step from Task 3 verbatim, so the wiki stops
drifting from the workflow file.

> [!note] Scope of reconciliation
> This plan only fixes the `Create release` step snippet — the wiki page has
> other pre-existing staleness (e.g. older `actions/checkout@v4` vs the
> current `@v5`) that predates this change. Fixing that is outside this
> plan's scope; flag it as a follow-up if needed.

- [ ] **Step 2: Append a log entry**

Append to `docs/log.md`:

```markdown
## [2026-04-09] maint | GitHub Actions Node 24 upgrade

Addressed the "Node.js 20 actions are deprecated" warnings surfaced by the
release workflow runs (deadline 2026-06-02):

- Bumped `actions/upload-artifact` and `actions/download-artifact` to `@v5`
  across `.github/workflows/release.yml` and `.github/workflows/ci.yml`
- Replaced `softprops/action-gh-release@v2` with an inline `gh release create`
  Bash step (upstream has no Node 24 release as of this date)
- Reconciled [[CD-plan/release-workflow|Release Workflow]] to reflect the new
  release step
- Plan recorded at [[specs/plans/2026-04-09-github-actions-node24-upgrade|GitHub Actions Node 24 Upgrade]]
```

- [ ] **Step 3: Add this plan to `docs/index.md`**

Add a row under the appropriate section (likely the CD-plan / specs row group)
of `docs/index.md`:

```markdown
| [[specs/plans/2026-04-09-github-actions-node24-upgrade|GitHub Actions Node 24 Upgrade]] | Plan: bump artifact actions to `@v5`, replace `softprops/action-gh-release` with `gh release create` | 2026-04-09 |
```

- [ ] **Step 4: Commit**

```bash
git add docs/CD-plan/release-workflow.md docs/log.md docs/index.md \
        docs/specs/plans/2026-04-09-github-actions-node24-upgrade.md
git commit -m "docs(cd): reconcile release-workflow wiki with Node 24 upgrade"
```

---

## Task 7: Push and verify on GitHub

**Files:** none — git + GitHub UI.

- [ ] **Step 1: Push the branch**

```bash
git push -u origin chore/gh-actions-node24
```

- [ ] **Step 2: Watch the CI run**

```bash
gh run watch --exit-status
```

Expected: CI run succeeds. Specifically check the "Annotations" panel in the
run summary — the "Node.js 20 actions are deprecated" warning should no longer
appear for the `Build Package` or `Test` jobs.

- [ ] **Step 3: Open a PR for review**

```bash
gh pr create --title "ci: upgrade deprecated Node 20 actions" \
  --body "$(cat <<'EOF'
## Summary

- Bumps `actions/upload-artifact` and `actions/download-artifact` to `@v5` (first Node 24 major) in both `release.yml` and `ci.yml`
- Replaces `softprops/action-gh-release@v2` with an inline `gh release create` step — upstream has no Node 24 release (see softprops/action-gh-release#742; PRs #670, #774 still open) and GitHub's deadline is 2026-06-02
- Reconciles the `CD-plan/release-workflow` wiki page with the new release step

## Test plan

- [ ] CI run on this branch is green
- [ ] The "Node.js 20 actions are deprecated" warning no longer appears in the CI run annotations
- [ ] A pre-release tag (`v0.0.0-test1`) dry-run on the release workflow succeeds end-to-end (optional — can be done post-merge on a disposable tag)
EOF
)"
```

> [!note] Release workflow smoke test
> The release workflow is only exercised on tag push, so the CI run on this
> branch will **not** validate the new `gh release create` step or the bumped
> `download-artifact@v5` steps. Two options:
>
> 1. Cut a disposable pre-release tag like `v0.0.0-nodetest1` on this branch
>    after merge, let it run, then delete the tag and the resulting GitHub
>    release if everything worked.
> 2. Defer the validation to the next real release tag and monitor closely.
>
> Option 1 is safer. Recommend it to the reviewer in the PR conversation.

---

## Rollback

If the upgrade causes failures:

- **Artifact action regression:** revert the specific `@v5` commit(s) and
  temporarily pin to `actions/upload-artifact@v4` / `@v4` with an explicit
  `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` env var at the job level — this
  keeps Node 24 but unblocks if v5 has an unexpected regression for us.
- **`gh release create` regression:** revert the Task 3 commit. As a fallback,
  set `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` at the `github-release` job
  level and keep `softprops/action-gh-release@v2` temporarily — the warning
  comes back but releases work until the upstream issue is resolved.

---

## See also

- [[CD-plan/release-workflow|Release Workflow]]
- [[CD-plan/README|CD Plan]]
- [GitHub changelog: Node 20 deprecation](https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/)
- [softprops/action-gh-release#742 (Node 20 tracking)](https://github.com/softprops/action-gh-release/issues/742)
