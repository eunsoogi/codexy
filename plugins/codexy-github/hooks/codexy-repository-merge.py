#!/usr/bin/python3
"""Repository-merge hook entrypoint."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

from codexy_policy.envelope import evaluate
from codexy_policy.repository_merge import forbidden

TOOLS = frozenset(
    {
        "mcp__codex_apps__github_merge_pull_request",
        "mcp__codex_apps__github_enable_auto_merge",
    }
)


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument(
        "--event", required=True, choices=("PreToolUse", "PermissionRequest")
    )
    event = parser.parse_args().event
    output = evaluate(
        event,
        sys.stdin.buffer.read(1024 * 1024 + 1),
        TOOLS,
        "CODEXY_REPOSITORY_MERGE_",
        forbidden,
    )
    if output:
        sys.stdout.buffer.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
