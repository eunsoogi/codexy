"""Command construction and execution for the repository lint runner."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

from lint_repository_inventory import (
    EXPECTED_LANGUAGES,
    LANGUAGES,
    Step,
    TEXT_EXCLUSIONS,
    inventory_files,
    selected_files,
    shell_files,
    shell_groups,
    tool_versions,
    validate_inventory,
)


def build_plan(root: Path, mode: str, selected: set[str]) -> list[Step]:
    validate_inventory(LANGUAGES)
    if not selected <= EXPECTED_LANGUAGES:
        raise ValueError("unknown language requested")
    checking, plan = mode == "check", []
    if "rust" in selected:
        fmt = (
            "cargo",
            "+1.85.0",
            "fmt",
            "--manifest-path",
            "packages/codexy-runtime/Cargo.toml",
            "--all",
        )
        plan.append(
            Step("rust", fmt + (("--", "--check") if checking else ()), checking)
        )
        plan.append(
            Step(
                "rust",
                (
                    "cargo",
                    "+1.85.0",
                    "clippy",
                    "--manifest-path",
                    "packages/codexy-runtime/Cargo.toml",
                    "--locked",
                    "--all-targets",
                    "--all-features",
                    "--",
                    "-D",
                    "warnings",
                ),
                True,
            )
        )
    if "python" in selected:
        files = inventory_files(root, "python")
        check = ("ruff", "check") if checking else ("ruff", "check", "--fix")
        plan += [
            Step("python", check + ("--", *files), checking),
            Step(
                "python",
                (("ruff", "format", "--check") if checking else ("ruff", "format"))
                + ("--", *files),
                checking,
            ),
        ]
    if "shell" in selected:
        files = shell_files(root)
        plan += [
            Step("shell", ("shellcheck", f"--shell={dialect}", "--", *paths), True)
            for dialect, paths in shell_groups(root).items()
        ]
        plan.append(
            Step("shell", ("shfmt", "-d" if checking else "-w", "--", *files), checking)
        )
    if "powershell" in selected:
        files, version = (
            inventory_files(root, "powershell"),
            tool_versions(root)["PSScriptAnalyzer"],
        )
        module = os.environ.get("CODEXY_PSSCRIPTANALYZER_PATH")
        module_arg = ("-ModulePath", module) if module else ()
        plan.append(
            Step(
                "powershell",
                (
                    "pwsh",
                    "-NoLogo",
                    "-NoProfile",
                    "-File",
                    "scripts/lint-powershell.ps1",
                    "-Mode",
                    f"--{mode}",
                    "-Version",
                    version,
                    *module_arg,
                    "-Path",
                    *files,
                ),
                checking,
            )
        )
    if "windows-command" in selected:
        plan.append(
            Step(
                "windows-command",
                (
                    sys.executable,
                    "scripts/lint-windows-command.py",
                    *inventory_files(root, "windows-command"),
                ),
                True,
            )
        )
    if "text" in selected:
        prettier = selected_files(
            root, ("*.md", "*.json", "*.yaml", "*.yml"), TEXT_EXCLUSIONS
        )
        toml = selected_files(root, ("*.toml",), TEXT_EXCLUSIONS)
        taplo = ("taplo", "fmt", "--check") if checking else ("taplo", "fmt")
        plan += [
            Step(
                "text",
                (
                    "npx",
                    "--no-install",
                    "prettier",
                    "--config",
                    ".prettierrc.json",
                    "--check" if checking else "--write",
                    "--",
                    *prettier,
                ),
                checking,
            ),
            Step("text", taplo + ("--", *toml), checking),
        ]
    return plan


def version_matches(output: str, expected: str) -> bool:
    tokens = re.findall(
        r"(?<![\w.])v?(\d+\.\d+\.\d+(?:(?:a|b|rc|dev|post)\d+|[-+][\w.-]+)?)(?![\w.])",
        output,
    )
    return expected in tokens


def verify_versions(root: Path, plan: list[Step]) -> bool:
    versions = tool_versions(root)
    checks = {
        "rust": [(("rustc", "+1.85.0", "--version"), "rust")],
        "python": [(("ruff", "--version"), "ruff")],
        "shell": [
            (("shellcheck", "--version"), "shellcheck"),
            (("shfmt", "--version"), "shfmt"),
        ],
        "text": [
            (("npx", "--no-install", "prettier", "--version"), "prettier"),
            (("taplo", "--version"), "taplo"),
        ],
    }
    languages = {step.language for step in plan}
    for language, commands in checks.items():
        if language not in languages:
            continue
        for command, tool in commands:
            try:
                output = subprocess.run(
                    command, text=True, check=True, capture_output=True
                ).stdout
            except (OSError, subprocess.CalledProcessError) as error:
                print(f"cannot verify {tool} version: {error}", file=sys.stderr)
                return False
            if not version_matches(output, versions[tool]):
                print(
                    f"{tool} version does not include pinned {versions[tool]}: {output}",
                    file=sys.stderr,
                )
                return False
    return True


def run(plan: list[Step], root: Path) -> int:
    if not verify_versions(root, plan):
        return 1
    for step in plan:
        print(f"[{step.language}] {json.dumps(step.command)}", flush=True)
        if subprocess.run(step.command, cwd=root, check=False).returncode:
            return 1
    return 0
