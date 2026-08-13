"""Public command line for transactional component lifecycle operations."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

from .component_lifecycle import run_operation
from .component_transaction_receipts import RECEIPT_SCHEMA


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="getcodexy", allow_abbrev=False)
    parser.add_argument("--codex", type=Path, help="optional absolute path supplied by the trusted Codex host")
    parser.add_argument("--codex-home", type=Path, default=Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")))
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("install", "update", "remove"):
        child = commands.add_parser(command, allow_abbrev=False)
        child.add_argument("components", nargs="*")
        child.add_argument("--json", action="store_true", dest="json_output")
    arguments = parser.parse_args(argv)
    try:
        receipt = run_operation(arguments.command, tuple(arguments.components), arguments.codex_home, arguments.codex)
    except Exception as error:
        print(f"getcodexy {arguments.command}: {error}", file=sys.stderr)
        return 1
    if arguments.json_output:
        print(json.dumps(receipt, sort_keys=True))
    else:
        print(f"getcodexy {receipt['command']}: {receipt['outcome']}")
    return 0 if receipt["outcome"] == "completed" else 2


if __name__ == "__main__":
    raise SystemExit(main())
