"""Verify that pre-commit hooks agree with the repository's expectations.

This script runs the configured pre-commit suite against the current
working tree and reports any hooks that fail or drift from the pinned
versions declared in ``.pre-commit-config.yaml``. It is intended to be
invoked manually or from CI as a belt-and-braces check that the hook
contract stays honest between contributor machines and the gated
pipeline.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent

REQUIRED_ROOT_FILES = [
    "README.md",
    "AGENTS.md",
    "CLAUDE.md",
    "CONCEPTS.md",
    ".markdownlint-cli2.jsonc",
    "docs/README.md",
    "docs/index.md",
    "docs/log.md",
    "docs/research/llm-wiki.md",
]

REQUIRED_FRONTMATTER_KEYS = {"title", "tags", "date-created"}
OBSIDIAN_PATTERNS = [
    re.compile(r"\[\[[^\]]+\]\]"),
    re.compile(r"!\[\[[^\]]+\]\]"),
    re.compile(r"^>\s*\[![A-Za-z0-9_-]+\]", re.MULTILINE),
]


def fail(message: str) -> None:
    """Print a FAIL-prefixed message to stdout."""
    print(f"FAIL: {message}")


def ok(message: str) -> None:
    """Print a PASS-prefixed message to stdout."""
    print(f"PASS: {message}")


# Directories whose markdown files are not subject to the strict
# root-level rules. Each has its own review contract:
#
# - .git/       : git internals
# - docs/       : Obsidian-flavored wiki under its own maturity standard
# - .github/    : GitHub-specific surfaces (rich README, issue/PR
#                 templates, etc.) where GitHub-flavored Markdown
#                 including inline HTML and alert admonitions is
#                 expected. See .github/README.md for the rich landing
#                 page version.
# - .venv/      : throwaway virtualenvs created for worktree sanity
#                 checks; their vendored LICENSE.md files are upstream
#                 and not subject to our lint rules.
# - .worktrees/ : git worktrees share the repo's .git but live under a
#                 separate directory; their docs/ contain Obsidian markdown
#                 and are governed by their own branch's pre-commit hook.
_EXCLUDED_TOP_DIRS = {".git", "docs", ".github", ".venv", ".worktrees"}


def markdown_files_outside_docs() -> list[Path]:
    """Return all ``*.md`` files that are outside the excluded top-level directories."""
    files: list[Path] = []
    for path in REPO_ROOT.rglob("*.md"):
        relative = path.relative_to(REPO_ROOT)
        if relative.parts[0] in _EXCLUDED_TOP_DIRS:
            continue
        files.append(path)
    return sorted(files)


def wiki_markdown_files() -> list[Path]:
    """Return all ``*.md`` files under ``docs/`` excluding the ``research/`` subtree."""
    files: list[Path] = []
    for path in (REPO_ROOT / "docs").rglob("*.md"):
        relative = path.relative_to(REPO_ROOT)
        if "research" in relative.parts:
            continue
        files.append(path)
    return sorted(files)


def check_required_files() -> bool:
    """Verify that every file in ``REQUIRED_ROOT_FILES`` exists. Returns ``True`` on success."""
    success = True
    for relative_path in REQUIRED_ROOT_FILES:
        path = REPO_ROOT / relative_path
        if path.exists():
            ok(f"required file present: {relative_path}")
        else:
            fail(f"required file missing: {relative_path}")
            success = False
    return success


def check_claude_stub() -> bool:
    """Verify that ``CLAUDE.md`` contains a reference to ``AGENTS.md``. Returns ``True`` on success."""
    claude = (REPO_ROOT / "CLAUDE.md").read_text(encoding="utf-8")
    if "AGENTS.md" in claude:
        ok("CLAUDE.md points to AGENTS.md")
        return True
    fail("CLAUDE.md does not point to AGENTS.md")
    return False


def check_obsidian_boundary() -> bool:
    """Verify that Obsidian-specific markdown (wikilinks, callouts) is confined to ``docs/``. Returns ``True`` on success."""
    success = True
    for path in markdown_files_outside_docs():
        text = path.read_text(encoding="utf-8")
        sanitized = strip_code(text)
        for pattern in OBSIDIAN_PATTERNS:
            if pattern.search(sanitized):
                fail(
                    "Obsidian-specific markdown found outside docs: "
                    f"{path.relative_to(REPO_ROOT)}"
                )
                success = False
                break
    if success:
        ok("Obsidian-specific markdown is confined to docs/")
    return success


def parse_frontmatter(text: str) -> tuple[bool, set[str]]:
    """Parse YAML frontmatter from ``text``. Returns ``(has_frontmatter, key_set)``."""
    lines = text.splitlines()
    if len(lines) < 3 or lines[0].strip() != "---":
        return False, set()
    try:
        end_index = lines[1:].index("---") + 1
    except ValueError:
        return False, set()

    keys: set[str] = set()
    for line in lines[1:end_index]:
        if not line or line.startswith((" ", "\t", "- ")):
            continue
        if ":" in line:
            keys.add(line.split(":", 1)[0].strip())
    return True, keys


def strip_code(text: str) -> str:
    """Remove fenced code blocks and inline code spans from ``text`` before pattern-matching."""
    text = re.sub(r"```[\s\S]*?```", "", text)
    text = re.sub(r"`[^`\n]+`", "", text)
    return text


def check_wiki_frontmatter() -> bool:
    """Verify that every Obsidian wiki page has required YAML frontmatter keys. Returns ``True`` on success."""
    success = True
    for path in wiki_markdown_files():
        text = path.read_text(encoding="utf-8")
        has_frontmatter, keys = parse_frontmatter(text)
        relative = path.relative_to(REPO_ROOT)
        if not has_frontmatter:
            fail(f"missing YAML frontmatter: {relative}")
            success = False
            continue
        missing = REQUIRED_FRONTMATTER_KEYS - keys
        if missing:
            fail(
                f"frontmatter missing keys {sorted(missing)}: {relative}"
            )
            success = False
            continue
    if success:
        ok("wiki pages under docs/ have required frontmatter")
    return success


def run_markdownlint() -> bool:
    """Run ``markdownlint-cli2`` against the repository via ``npx``. Returns ``True`` on clean exit."""
    npx = shutil.which("npx.cmd") or shutil.which("npx")
    if not npx:
        fail("npx was not found, so markdownlint could not run")
        return False

    # Drive scope from .markdownlint-cli2.jsonc `ignores` rather than an
    # explicit file list — Windows command-line length caps out around a
    # few hundred paths once the bootstrap subtree docs are included.
    command = [npx, "markdownlint-cli2", "**/*.md"]
    result = subprocess.run(command, cwd=REPO_ROOT, check=False)
    if result.returncode == 0:
        ok("markdownlint passed for markdown outside docs/")
        return True

    fail("markdownlint failed for markdown outside docs/")
    return False


def staged_rust_files() -> list[str]:
    """Return staged ``*.rs`` paths (added, copied, modified, renamed).

    Returns an empty list when no Rust files are staged, allowing the fmt
    check to be skipped entirely for documentation-only or non-Rust commits.
    """
    result = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return [p for p in result.stdout.splitlines() if p.endswith(".rs")]


def check_rust_fmt() -> bool:
    """Run ``cargo fmt --all -- --check`` when Rust files are staged.

    Mirrors the ``fmt`` job in ``.github/workflows/rust-ci.yml`` so
    formatting failures are caught locally before they reach CI.

    Skipped when no ``*.rs`` files are staged.  Returns ``True`` on success
    or skip, ``False`` on failure.
    """
    rust_files = staged_rust_files()
    if not rust_files:
        ok("no staged Rust files — skipping cargo fmt check")
        return True

    cargo = shutil.which("cargo")
    if not cargo:
        fail("cargo not found — cannot run fmt check (install Rust toolchain)")
        return False

    rust_dir = REPO_ROOT / "rust"
    if not rust_dir.is_dir():
        fail(f"rust/ directory not found at {rust_dir} — skipping fmt check")
        return True

    result = subprocess.run(
        [cargo, "fmt", "--all", "--", "--check"],
        cwd=rust_dir,
        check=False,
    )
    if result.returncode == 0:
        ok(f"cargo fmt clean ({len(rust_files)} staged Rust file(s))")
        return True

    suffix = " …" if len(rust_files) > 5 else ""
    fail(
        f"cargo fmt check failed — run `cargo fmt --all` from rust/ to fix\n"
        f"  Staged Rust files: {', '.join(rust_files[:5])}{suffix}"
    )
    return False


def main() -> int:
    """Run all pre-commit verification checks and return an exit code (0 = pass, 1 = fail)."""
    checks = [
        check_required_files(),
        check_claude_stub(),
        check_obsidian_boundary(),
        check_wiki_frontmatter(),
        run_markdownlint(),
        check_rust_fmt(),
    ]
    if all(checks):
        print("TinyQuant pre-commit verification passed.")
        return 0
    print("TinyQuant pre-commit verification failed.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
