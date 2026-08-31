#!/usr/bin/env python3
"""Check explicit governed files without depending on a checkout root."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


MAX_PHYSICAL_LINES = 250


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--path",
        dest="paths",
        action="append",
        required=True,
        help="explicit governed file path; repeat for each applicable file",
    )
    return parser.parse_args()


def line_count(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines())


def main() -> int:
    args = arguments()
    maximum = MAX_PHYSICAL_LINES
    results = []
    violations = []
    for raw_path in args.paths:
        path = Path(raw_path)
        try:
            if not path.is_file() or path.is_symlink():
                raise ValueError("path must be a regular file")
            count = line_count(path)
            result = {"path": str(path), "lines": count, "limit": maximum}
            if count > maximum:
                result["status"] = "fail"
                violations.append(result)
            else:
                result["status"] = "pass"
            results.append(result)
        except (OSError, UnicodeError, ValueError) as error:
            result = {"path": str(path), "status": "error", "error": str(error)}
            results.append(result)
            violations.append(result)

    status = "pass" if not violations else "fail"
    print(json.dumps({"status": status, "policy": "governed-code.v1", "files": results}))
    return 0 if not violations else 1


if __name__ == "__main__":
    sys.exit(main())
