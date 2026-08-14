#!/usr/bin/env python3
"""Run real valid, invalid, and idempotence fixtures for one lint route."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/lint-fixtures"


def command(*args: str, succeeds: bool) -> None:
    completed = subprocess.run(args, cwd=ROOT, check=False)
    if (completed.returncode == 0) != succeeds:
        expected = "pass" if succeeds else "fail"
        raise SystemExit(f"expected {expected}: {args}")


def copied(relative: str) -> Path:
    directory = Path(tempfile.mkdtemp(prefix="codexy-lint-fixture-"))
    source, target = FIXTURES / relative, directory / Path(relative).name
    if source.is_dir():
        shutil.copytree(source, target)
    else:
        shutil.copy2(source, target)
    return target


def idempotent(
    formatter: tuple[str, ...], checker: tuple[str, ...], target: Path
) -> None:
    command(*formatter, str(target), succeeds=True)
    once = target.read_bytes()
    command(*formatter, str(target), succeeds=True)
    if target.read_bytes() != once:
        raise SystemExit(f"formatter is not idempotent: {target.name}")
    command(*checker, str(target), succeeds=True)


def rust() -> None:
    bad, good = FIXTURES / "rust/bad/Cargo.toml", FIXTURES / "rust/good/Cargo.toml"
    command(
        "cargo",
        "+1.85.0",
        "fmt",
        "--manifest-path",
        str(bad),
        "--",
        "--check",
        succeeds=False,
    )
    command(
        "cargo",
        "+1.85.0",
        "clippy",
        "--manifest-path",
        str(bad),
        "--locked",
        "--",
        "-D",
        "warnings",
        succeeds=False,
    )
    command(
        "cargo",
        "+1.85.0",
        "clippy",
        "--manifest-path",
        str(good),
        "--locked",
        "--",
        "-D",
        "warnings",
        succeeds=True,
    )
    target = copied("rust/bad")
    manifest = target / "Cargo.toml"
    command("cargo", "+1.85.0", "fmt", "--manifest-path", str(manifest), succeeds=True)
    first = (target / "src/main.rs").read_bytes()
    command("cargo", "+1.85.0", "fmt", "--manifest-path", str(manifest), succeeds=True)
    if (target / "src/main.rs").read_bytes() != first:
        raise SystemExit("rustfmt is not idempotent")
    command(
        "cargo",
        "+1.85.0",
        "fmt",
        "--manifest-path",
        str(manifest),
        "--",
        "--check",
        succeeds=True,
    )


def python() -> None:
    command("ruff", "check", "--", str(FIXTURES / "python/bad.py"), succeeds=False)
    command("ruff", "check", "--", str(FIXTURES / "python/good.py"), succeeds=True)
    target = copied("python/fix.py")
    idempotent(("ruff", "format"), ("ruff", "format", "--check"), target)
    command("ruff", "check", "--fix", "--", str(target), succeeds=True)
    command("ruff", "check", "--", str(target), succeeds=True)


def shell() -> None:
    command(
        "shellcheck", "--shell=sh", "--", str(FIXTURES / "shell/bad.sh"), succeeds=False
    )
    command(
        "shellcheck", "--shell=sh", "--", str(FIXTURES / "shell/good.sh"), succeeds=True
    )
    target = copied("shell/fix.sh")
    idempotent(("shfmt", "-w", "--"), ("shfmt", "-d", "--"), target)


def text() -> None:
    prettier = ("npx", "--no-install", "prettier", "--config", ".prettierrc.json")
    command(*prettier, "--check", "--", str(FIXTURES / "text/bad.json"), succeeds=False)
    command(*prettier, "--check", "--", str(FIXTURES / "text/good.json"), succeeds=True)
    markdown = copied("text/fix.md")
    idempotent(prettier + ("--write", "--"), prettier + ("--check", "--"), markdown)
    command(
        "taplo", "fmt", "--check", "--", str(FIXTURES / "text/bad.toml"), succeeds=False
    )
    command(
        "taplo", "fmt", "--check", "--", str(FIXTURES / "text/good.toml"), succeeds=True
    )
    toml = copied("text/fix.toml")
    idempotent(("taplo", "fmt", "--"), ("taplo", "fmt", "--check", "--"), toml)


def powershell() -> None:
    version = "1.25.0"
    module = os.environ.get("CODEXY_PSSCRIPTANALYZER_PATH")
    base = (
        "pwsh",
        "-NoLogo",
        "-NoProfile",
        "-File",
        "scripts/lint-powershell.ps1",
        "-Version",
        version,
    )
    module_arg = ("-ModulePath", module) if module else ()
    command(
        *base,
        "-Mode",
        "--check",
        *module_arg,
        "-Path",
        "tests/lint-fixtures/powershell/bad.ps1",
        succeeds=False,
    )
    command(
        *base,
        "-Mode",
        "--check",
        *module_arg,
        "-Path",
        "tests/lint-fixtures/powershell/good.ps1",
        succeeds=True,
    )
    multi_path = subprocess.run(
        [
            *base,
            "-Mode",
            "--check",
            *module_arg,
            "-Path",
            "tests/lint-fixtures/powershell/good.ps1",
            "tests/lint-fixtures/powershell/bad.ps1",
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    multi_path_output = multi_path.stdout + multi_path.stderr
    if multi_path.returncode == 0 or "bad.ps1" not in multi_path_output:
        raise SystemExit("expected multi-path PowerShell analysis to reject bad.ps1")
    if "A positional parameter cannot be found" in multi_path_output:
        raise SystemExit("PowerShell did not bind every lint path")
    directory = Path(tempfile.mkdtemp(dir=ROOT, prefix=".lint-fixture-"))
    target = directory / "fix.ps1"
    shutil.copy2(FIXTURES / "powershell/fix.ps1", target)
    relative = target.relative_to(ROOT).as_posix()
    try:
        command(*base, "-Mode", "--fix", *module_arg, "-Path", relative, succeeds=True)
        once = target.read_bytes()
        command(*base, "-Mode", "--fix", *module_arg, "-Path", relative, succeeds=True)
        if target.read_bytes() != once:
            raise SystemExit("Invoke-Formatter is not idempotent")
        command(
            *base, "-Mode", "--check", *module_arg, "-Path", relative, succeeds=True
        )
    finally:
        shutil.rmtree(directory)


def windows_command() -> None:
    checker = (sys.executable, "scripts/lint-windows-command.py")
    command(*checker, "tests/lint-fixtures/windows-command/bad.cmd", succeeds=False)
    command(*checker, "tests/lint-fixtures/windows-command/valid.cmd", succeeds=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--language",
        required=True,
        choices=("rust", "python", "shell", "text", "powershell", "windows-command"),
    )
    globals()[parser.parse_args().language.replace("-", "_")]()


if __name__ == "__main__":
    main()
