#!/usr/bin/env python3
"""Run the repository's language-complete checks or safe formatters."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).parent))
from lint_repository_inventory import (  # noqa: E402, F401
    EXPECTED_LANGUAGES,
    LANGUAGES,
    Step,
    inventory_files,
    shebang_language,
    shebang_inventory,
    tool_versions,
    tracked_regular_files,
    validate_inventory,
)
from lint_repository_plan import (  # noqa: E402, F401
    build_plan,
    run,
    verify_versions,
    version_matches,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--check", action="store_true", help="check without modifying source files"
    )
    mode.add_argument(
        "--fix", action="store_true", help="apply only safe formatters and fixers"
    )
    parser.add_argument(
        "--language", action="append", choices=sorted(EXPECTED_LANGUAGES)
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[1]
    mode = "check" if args.check else "fix"
    return run(build_plan(root, mode, set(args.language or EXPECTED_LANGUAGES)), root)


if __name__ == "__main__":
    raise SystemExit(main())
