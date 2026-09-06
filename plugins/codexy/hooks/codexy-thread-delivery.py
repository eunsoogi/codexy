#!/usr/bin/python3
# pyright: reportImplicitRelativeImport=false
"""Thread-delivery hook entrypoint."""

import argparse
import os
import sys

if os.environ.get("CODEXY_HOOK_SILENT") == "1":
    sys.stderr = open(os.devnull, "w", encoding="utf-8")

UNSUPPORTED_INTERPRETER_EXIT = 125
if sys.version_info < (3, 10):
    raise SystemExit(UNSUPPORTED_INTERPRETER_EXIT)

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

TOOLS = frozenset(
    {"codex_app__send_message_to_thread", "mcp__codex_app__send_message_to_thread"}
)
TIMING_FILE_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"


def main() -> int:
    from codexy_policy.envelope import evaluate
    from codexy_policy.thread_delivery import forbidden

    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument(
        "--event", required=True, choices=("PreToolUse", "PermissionRequest")
    )
    event = parser.parse_args().event
    payload = sys.stdin.buffer.read(1024 * 1024 + 1)
    timing = None
    started_ns = None
    if os.environ.get(TIMING_FILE_ENV):
        try:
            from codexy_policy import timing as timing_module

            timing = timing_module
            started_ns = timing.start()
        except Exception:
            timing = None
    output = evaluate(event, payload, TOOLS, "CODEXY_THREAD_DELIVERY_", forbidden)
    if timing is not None and started_ns is not None:
        timing.record(
            event,
            "thread-delivery",
            started_ns,
            "deny" if output else "allow",
        )
    if output:
        sys.stdout.buffer.write(output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SystemExit as error:
        if error.code == UNSUPPORTED_INTERPRETER_EXIT:
            raise SystemExit(1) from error
        raise
