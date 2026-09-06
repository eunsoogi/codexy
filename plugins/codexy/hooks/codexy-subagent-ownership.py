#!/usr/bin/python3
# pyright: reportImplicitRelativeImport=false
"""Subagent role admission hook."""

import argparse
import os
import sys
from typing import cast

if os.environ.get("CODEXY_HOOK_SILENT") == "1":
    sys.stderr = open(os.devnull, "w", encoding="utf-8")

UNSUPPORTED_INTERPRETER_EXIT = 125
if sys.version_info < (3, 10):
    raise SystemExit(UNSUPPORTED_INTERPRETER_EXIT)

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

TOOLS = frozenset({"spawn_agent", "agents__spawn_agent", "multi_agent_v1__spawn_agent"})


def main() -> int:
    from codexy_policy.envelope import evaluate
    from codexy_policy.subagent_ownership import forbidden

    parser = argparse.ArgumentParser(allow_abbrev=False)
    _ = parser.add_argument(
        "--event", required=True, choices=("PreToolUse", "PermissionRequest")
    )
    event = cast(str, parser.parse_args().event)
    output = evaluate(
        event,
        sys.stdin.buffer.read(1024 * 1024 + 1),
        TOOLS,
        "CODEXY_SUBAGENT_OWNERSHIP_",
        forbidden,
    )
    if output:
        _ = sys.stdout.buffer.write(output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SystemExit as error:
        if error.code == UNSUPPORTED_INTERPRETER_EXIT:
            raise SystemExit(1) from error
        raise
