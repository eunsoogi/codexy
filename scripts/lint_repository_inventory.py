"""NUL-safe tracked-file inventory for the repository lint runner."""

from __future__ import annotations

import json
import re
import stat
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple


EXPECTED_LANGUAGES = frozenset(
    {"rust", "python", "shell", "powershell", "windows-command", "text"}
)
SHELL_INTERPRETERS = {b"sh", b"bash", b"zsh", b"dash", b"ksh"}
SOURCE_PATTERNS = {
    "rust": (
        "*.rs",
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "rustfmt.toml",
        "clippy.toml",
    ),
    "python": ("*.py",),
    "powershell": ("*.ps1",),
    "windows-command": ("*.cmd",),
    "text": ("*.md", "*.json", "*.yaml", "*.yml", "*.toml"),
}
TEXT_EXCLUSIONS = (
    "packages/codexy-runtime/tests/fixtures/",
    "packages/codexy-runtime/tests/mcp/fixtures/",
    "packages/getcodexy/tests/fixtures/",
    "plugins/codexy-devtools/runtime/",
)
LANGUAGES = {
    "rust": {"check": "rustfmt and clippy", "fix": "rustfmt", "fixable": True},
    "python": {"check": "ruff", "fix": "ruff", "fixable": True},
    "shell": {"check": "shellcheck and shfmt", "fix": "shfmt", "fixable": True},
    "powershell": {
        "check": "PSScriptAnalyzer",
        "fix": "Invoke-Formatter",
        "fixable": True,
    },
    "windows-command": {
        "check": "safe static launcher validation",
        "fix": None,
        "fixable": False,
    },
    "text": {"check": "Prettier", "fix": "Prettier", "fixable": True},
}


class Step(NamedTuple):
    language: str
    command: tuple[str, ...]
    read_only: bool


def tool_versions(root: Path) -> dict[str, str]:
    policy = json.loads((root / "tooling/lint-tools.json").read_text(encoding="utf-8"))
    return {
        "rust": policy["rust"],
        "prettier": policy["prettier"],
        "ruff": policy["ruff"],
        "shellcheck": policy["shellcheck"]["version"],
        "shfmt": policy["shfmt"]["version"],
        "taplo": policy["taplo"]["version"],
        "PSScriptAnalyzer": policy["PSScriptAnalyzer"],
    }


def validate_inventory(inventory: dict[str, dict[str, object]]) -> None:
    if set(inventory) != EXPECTED_LANGUAGES:
        missing, extra = (
            sorted(EXPECTED_LANGUAGES - set(inventory)),
            sorted(set(inventory) - EXPECTED_LANGUAGES),
        )
        raise ValueError(
            f"language inventory mismatch: missing={missing}, extra={extra}"
        )
    for name, policy in inventory.items():
        if not policy.get("check") or "fixable" not in policy:
            raise ValueError(f"language inventory is incomplete: {name}")


def tracked_regular_files(
    root: Path, *patterns: str, excluded: tuple[str, ...] = ()
) -> tuple[str, ...]:
    listed = subprocess.run(
        ["git", "ls-files", "-z", "--", *patterns],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    files: list[str] = []
    for encoded in filter(None, listed):
        relative = Path(encoded.decode(sys.getfilesystemencoding(), "surrogateescape"))
        if (
            relative.is_absolute()
            or ".." in relative.parts
            or relative.name.startswith("-")
        ):
            raise ValueError(f"unsafe tracked path: {relative}")
        if any(relative.as_posix().startswith(prefix) for prefix in excluded):
            continue
        if any(ord(character) < 32 for character in relative.as_posix()):
            raise ValueError(f"unsafe tracked path: {relative}")
        candidate, cursor = root / relative, root
        for part in relative.parts:
            cursor /= part
            if cursor.is_symlink():
                raise ValueError(f"tracked path must not cross a symlink: {relative}")
        if not stat.S_ISREG(candidate.lstat().st_mode):
            raise ValueError(f"tracked path must be a regular file: {relative}")
        if not candidate.resolve().is_relative_to(root.resolve()):
            raise ValueError(f"tracked path escapes repository: {relative}")
        files.append(relative.as_posix())
    return tuple(files)


def selected_files(
    root: Path, patterns: tuple[str, ...], excluded: tuple[str, ...] = ()
) -> tuple[str, ...]:
    return tracked_regular_files(root, *patterns, excluded=excluded)


def interpreter(first: bytes) -> bytes:
    words = first[2:].strip().split()
    if not words:
        return b""
    command = words[0].rsplit(b"/", 1)[-1]
    if command != b"env":
        return command
    arguments = words[1:]
    if arguments[:1] == [b"-S"]:
        arguments = arguments[1:]
    return next(
        (item.rsplit(b"/", 1)[-1] for item in arguments if not item.startswith(b"-")),
        b"",
    )


def shebang_language(first: bytes) -> str | None:
    if interpreter(first) in SHELL_INTERPRETERS:
        return "shell"
    if re.fullmatch(rb"python(?:\d+(?:\.\d+)*)?", interpreter(first)):
        return "python"
    return None


def shebang_inventory(root: Path) -> dict[str, tuple[str, ...]]:
    languages = {"shell": [], "python": []}
    for name in selected_files(root, ("*",), TEXT_EXCLUSIONS):
        first = (root / name).read_bytes().splitlines()[:1]
        if not first or not first[0].startswith(b"#!") or first[0].startswith(b"#!["):
            continue
        language = shebang_language(first[0])
        if language is None:
            raise ValueError(f"unclassified maintained shebang: {name}")
        languages[language].append(name)
    return {language: tuple(paths) for language, paths in languages.items()}


def shell_files(root: Path) -> tuple[str, ...]:
    return tuple(
        sorted(
            set(selected_files(root, ("*.sh",))) | set(shebang_inventory(root)["shell"])
        )
    )


def shell_groups(root: Path) -> dict[str, tuple[str, ...]]:
    groups: dict[str, list[str]] = {}
    for name in shell_files(root):
        shell = interpreter((root / name).read_bytes().splitlines()[0])
        dialect = shell.decode() if shell in SHELL_INTERPRETERS else "sh"
        groups.setdefault(dialect, []).append(name)
    return {dialect: tuple(files) for dialect, files in groups.items()}


def inventory_files(root: Path, language: str) -> tuple[str, ...]:
    if language not in EXPECTED_LANGUAGES:
        raise ValueError(f"unknown language: {language}")
    if language == "shell":
        return shell_files(root)
    if language == "python":
        return tuple(
            sorted(
                set(selected_files(root, SOURCE_PATTERNS[language]))
                | set(shebang_inventory(root)["python"])
            )
        )
    return selected_files(
        root, SOURCE_PATTERNS[language], TEXT_EXCLUSIONS if language == "text" else ()
    )
