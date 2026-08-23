#!/usr/bin/python3
"""Child-thread creation native pair admission hook."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

from codexy_policy.child_thread_creation import forbidden
from codexy_policy.envelope import evaluate

TOOLS = frozenset({"codex_app__create_thread"})


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
        "CODEXY_CHILD_THREAD_CREATION_",
        forbidden,
    )
    if output:
        sys.stdout.buffer.write(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
