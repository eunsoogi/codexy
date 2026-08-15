"""Receipt provenance and terminal phase reporting for Rust profiling."""

from __future__ import annotations

import argparse
import os
import subprocess
from pathlib import Path

from profile_rust_accounting import deadline_report_lines, declared_test_target_order


def receipt_head(root: Path) -> str:
    return subprocess.check_output(
        ("git", "rev-parse", "HEAD"), cwd=root, text=True
    ).strip()


def receipt_index_tree(root: Path) -> str:
    return subprocess.check_output(("git", "write-tree"), cwd=root, text=True).strip()


def runtime_package_root(root: Path) -> Path:
    root = root.resolve()
    nested_runtime = root / "packages" / "codexy-runtime"
    if nested_runtime.joinpath("Cargo.toml").is_file():
        return nested_runtime
    if root.joinpath("Cargo.toml").is_file():
        return root
    raise ValueError(
        "--root must name a Rust package root or a repository containing packages/codexy-runtime"
    )


def github_receipt_provenance() -> tuple[int, int]:
    values = tuple(
        os.environ.get(name) for name in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")
    )
    if any(
        value is None or not value.isascii() or not value.isdecimal() or int(value) < 1
        for value in values
    ):
        raise ValueError(
            "GITHUB_RUN_ID and GITHUB_RUN_ATTEMPT must be positive integers"
        )
    return int(values[0]), int(values[1])


def print_phases(
    arguments: argparse.Namespace,
    root: Path,
    output: str,
    status: int,
    phases: dict[str, float | str | Path],
    total_elapsed: float,
) -> None:
    for name, value in (
        ("child-status", status),
        ("windows-job-active-zero", phases["windows-job-active-zero"]),
        ("cargo-root-status", phases["cargo-root-status"]),
        ("windows-job-pids-json", phases["windows-job-pids-json"]),
        ("windows-job-images-json", phases["windows-job-images-json"]),
    ):
        print(f"{name}\t{value}")
    if status == 124:
        manifest = __import__("tomllib").loads((root / "Cargo.toml").read_text())
        print(
            *deadline_report_lines(output, declared_test_target_order(manifest)),
            f"deadline-linux-cargo-descendants-json\t{phases['linux-cargo-descendants-json']}",
            sep="\n",
        )
    for phase in ("workload", "capture", "replay", "inventory", "accounting"):
        print(f"phase-{phase}-seconds\t{phases[f'{phase}-seconds']:.3f}")
    print(f"total-seconds\t{total_elapsed:.3f}")
