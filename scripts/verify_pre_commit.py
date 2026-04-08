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
    print(f"FAIL: {message}")


def ok(message: str) -> None:
    print(f"PASS: {message}")


def markdown_files_outside_docs() -> list[Path]:
    files: list[Path] = []
    for path in REPO_ROOT.rglob("*.md"):
        relative = path.relative_to(REPO_ROOT)
        if relative.parts[0] in {".git", "docs"}:
            continue
        files.append(path)
    return sorted(files)


def wiki_markdown_files() -> list[Path]:
    files: list[Path] = []
    for path in (REPO_ROOT / "docs").rglob("*.md"):
        relative = path.relative_to(REPO_ROOT)
        if "research" in relative.parts:
            continue
        files.append(path)
    return sorted(files)


def check_required_files() -> bool:
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
    claude = (REPO_ROOT / "CLAUDE.md").read_text(encoding="utf-8")
    if "AGENTS.md" in claude:
        ok("CLAUDE.md points to AGENTS.md")
        return True
    fail("CLAUDE.md does not point to AGENTS.md")
    return False


def check_obsidian_boundary() -> bool:
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
    text = re.sub(r"```[\s\S]*?```", "", text)
    text = re.sub(r"`[^`\n]+`", "", text)
    return text


def check_wiki_frontmatter() -> bool:
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
    files = markdown_files_outside_docs()
    if not files:
        ok("no markdown files outside docs/ to lint")
        return True

    npx = shutil.which("npx.cmd") or shutil.which("npx")
    if not npx:
        fail("npx was not found, so markdownlint could not run")
        return False

    command = [npx, "markdownlint-cli2", *[str(path) for path in files]]
    result = subprocess.run(command, cwd=REPO_ROOT, check=False)
    if result.returncode == 0:
        ok("markdownlint passed for markdown outside docs/")
        return True

    fail("markdownlint failed for markdown outside docs/")
    return False


def main() -> int:
    checks = [
        check_required_files(),
        check_claude_stub(),
        check_obsidian_boundary(),
        check_wiki_frontmatter(),
        run_markdownlint(),
    ]
    if all(checks):
        print("TinyQuant pre-commit verification passed.")
        return 0
    print("TinyQuant pre-commit verification failed.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
