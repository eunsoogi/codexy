#!/usr/bin/env python3
"""Check tracked source with standard language tools, or apply safe fixes."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


LANGUAGES = ("rust", "python", "shell", "powershell", "windows-command", "text")
EXCLUDED_PREFIXES = (
    "packages/codexy-runtime/tests/fixtures/",
    "packages/codexy-runtime/tests/mcp/fixtures/",
    "packages/getcodexy/tests/fixtures/",
    "plugins/codexy-devtools/runtime/",
)


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--fix", action="store_true")
    parser.add_argument("--language", action="append", choices=LANGUAGES)
    return parser.parse_args()


def tracked(root: Path) -> list[str]:
    output = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    ).stdout
    changed_since = os.environ.get("CODEXY_LINT_CHANGED_SINCE")
    changed = None
    if changed_since:
        changed = set(
            filter(
                None,
                subprocess.run(
                    ["git", "diff", "--name-only", "-z", f"{changed_since}...HEAD"],
                    cwd=root,
                    check=True,
                    capture_output=True,
                )
                .stdout.decode("utf-8", "surrogateescape")
                .split("\0"),
            )
        )
    files = []
    for raw in filter(None, output.split(b"\0")):
        name = raw.decode("utf-8", "surrogateescape")
        path = Path(name)
        candidate = root / path
        if (
            (changed is not None and name not in changed)
            or name.startswith(EXCLUDED_PREFIXES)
            or path.is_absolute()
            or ".." in path.parts
            or path.name.startswith("-")
            or candidate.is_symlink()
            or not candidate.is_file()
        ):
            continue
        files.append(name)
    return files


def shebang(path: Path) -> str:
    try:
        return path.read_bytes().splitlines()[0].decode("utf-8", "ignore").lower()
    except IndexError:
        return ""


def inventory(root: Path) -> dict[str, list[str]]:
    groups = {language: [] for language in LANGUAGES}
    for name in tracked(root):
        path, suffix = root / name, Path(name).suffix.lower()
        header = shebang(path) if not suffix else ""
        if suffix == ".rs":
            groups["rust"].append(name)
        elif suffix == ".py" or "python" in header:
            groups["python"].append(name)
        elif suffix == ".sh" or any(
            shell in header for shell in ("/sh", "bash", "zsh", "dash", "ksh")
        ):
            groups["shell"].append(name)
        elif suffix == ".ps1":
            groups["powershell"].append(name)
        elif suffix == ".cmd":
            groups["windows-command"].append(name)
        elif suffix in {".md", ".json", ".yaml", ".yml", ".toml"}:
            groups["text"].append(name)
    return groups


def command(root: Path, *args: str) -> int:
    print("[lint]", " ".join(args), flush=True)
    return subprocess.run(args, cwd=root, check=False).returncode


def clippy(root: Path, paths: list[str]) -> int:
    completed = subprocess.run(
        (
            "cargo",
            "+1.85.0",
            "clippy",
            "--manifest-path",
            "packages/codexy-runtime/Cargo.toml",
            "--locked",
            "--all-targets",
            "--all-features",
            "--message-format=json",
            "--",
            "--cap-lints=warn",
        ),
        cwd=root,
        capture_output=True,
        text=True,
    )
    changed = set(paths)
    findings = []
    for line in completed.stdout.splitlines():
        try:
            message = json.loads(line).get("message", {})
        except json.JSONDecodeError:
            continue
        span = next(
            (span for span in message.get("spans", []) if span.get("is_primary")), {}
        )
        file_name = str(span.get("file_name", ""))
        relative = (Path("packages/codexy-runtime") / file_name).as_posix()
        if message.get("level") in {"warning", "error"} and relative in changed:
            findings.append(
                f"{file_name}:{span.get('line_start', '?')}: {message.get('message')}"
            )
    if completed.returncode:
        print(completed.stderr, file=sys.stderr, end="")
        return completed.returncode
    if findings:
        print("\n".join(findings), file=sys.stderr)
        return 1
    return 0


def check_cmd(root: Path, paths: list[str]) -> int:
    invalid = []
    for name in paths:
        text = (root / name).read_text(encoding="utf-8").lower()
        if not text.startswith("@echo off") or "exit /b" not in text:
            invalid.append(name)
    if invalid:
        print("invalid cmd launchers: " + ", ".join(invalid), file=sys.stderr)
        return 1
    return 0


def run(root: Path, mode: str, selected: set[str]) -> int:
    files = inventory(root)
    for language in selected:
        paths = files[language]
        if not paths:
            continue
        checking = mode == "check"
        if language == "rust":
            fmt = ["rustfmt", "+1.85.0", "--edition", "2024"]
            if checking:
                fmt.append("--check")
            if command(root, *fmt, "--", *paths) or (checking and clippy(root, paths)):
                return 1
        elif language == "python":
            check = ["ruff", "check"] + ([] if checking else ["--fix"])
            if command(root, *check, "--", *paths):
                return 1
            format_command = ["ruff", "format"] + (["--check"] if checking else [])
            if command(root, *format_command, "--", *paths):
                return 1
        elif language == "shell":
            if command(root, "shellcheck", "--", *paths) or command(
                root, "shfmt", "-d" if checking else "-w", "--", *paths
            ):
                return 1
        elif language == "powershell":
            if command(
                root,
                "pwsh",
                "-NoLogo",
                "-NoProfile",
                "-File",
                "scripts/lint-powershell.ps1",
                "-Mode",
                f"--{mode}",
                *paths,
            ):
                return 1
        elif language == "windows-command" and check_cmd(root, paths):
            return 1
        elif language == "text":
            prettier = [path for path in paths if Path(path).suffix != ".toml"]
            toml = [path for path in paths if Path(path).suffix == ".toml"]
            if prettier and command(
                root,
                "npx",
                "--no-install",
                "prettier",
                "--config",
                ".prettierrc.json",
                "--check" if checking else "--write",
                "--",
                *prettier,
            ):
                return 1
            taplo = ["taplo", "fmt"] + (["--check"] if checking else [])
            if toml and command(root, *taplo, "--", *toml):
                return 1
    return 0


def main() -> int:
    args = arguments()
    mode = "check" if args.check else "fix"
    return run(
        Path(__file__).resolve().parents[1], mode, set(args.language or LANGUAGES)
    )


if __name__ == "__main__":
    raise SystemExit(main())
