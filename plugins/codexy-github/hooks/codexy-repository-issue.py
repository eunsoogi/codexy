#!/usr/bin/python3
"""Repository-issue hook entrypoint."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

from codexy_policy.envelope import evaluate
from codexy_policy.repository_issue import forbidden

TOOLS = frozenset(
    {"mcp__codex_apps__github_create_issue", "mcp__codex_apps__github_update_issue"}
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
        "CODEXY_REPOSITORY_ISSUE_",
        forbidden,
    )
    if output:
        sys.stdout.buffer.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
