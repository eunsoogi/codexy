#!/usr/bin/python3
"""Native title checks without a general GitHub mutation admission policy."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

from codexy_policy.envelope import evaluate
from codexy_policy.title_check import TOOLS, forbidden


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument(
        "--event", required=True, choices=("PreToolUse", "PermissionRequest")
    )
    parser.add_argument("--kind", required=True, choices=tuple(TOOLS))
    args = parser.parse_args()
    output = evaluate(
        args.event,
        sys.stdin.buffer.read(1024 * 1024 + 1),
        TOOLS[args.kind],
        "CODEXY_TITLE_CHECK_",
        lambda request: forbidden(request, args.kind),
    )
    if output:
        sys.stdout.buffer.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
