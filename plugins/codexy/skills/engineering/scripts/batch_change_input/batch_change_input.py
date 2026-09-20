#!/usr/bin/env python3
"""Validate a local batch-change manifest and print a read-only preview."""

from __future__ import annotations

import argparse
import json
import sys

sys.dont_write_bytecode = True

from manifest import InputError, preview_from_path


SCHEMA = "codexy.batch-change-input.v1"


def _arguments(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--workspace-root", required=True, help="real workspace directory"
    )
    parser.add_argument(
        "--input", required=True, help="JSON input list path, or - for stdin"
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    try:
        arguments = _arguments(argv)
        result = preview_from_path(arguments.workspace_root, arguments.input)
    except InputError as error:
        print(
            json.dumps(
                {"schema": SCHEMA, "status": "error", "error": str(error)},
                sort_keys=True,
            )
        )
        return 2
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
