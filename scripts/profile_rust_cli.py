"""CLI argument construction for Rust profiling."""

import argparse
from pathlib import Path

from profile_rust_contract import BUDGET_SECONDS


def parse_arguments(
    description: str,
) -> tuple[argparse.ArgumentParser, argparse.Namespace]:
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "packages/codexy-runtime",
    )
    parser.add_argument("--workflow-file", type=Path)
    parser.add_argument("--budget-seconds", type=float, default=BUDGET_SECONDS)
    parser.add_argument("--verify-coverage", action="store_true")
    parser.add_argument("--windows", action="store_true")
    parser.add_argument("--shard")
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--aggregate-receipts", type=Path)
    parser.add_argument("--aggregate-platform", choices=("posix", "windows"))
    return parser, parser.parse_args()
