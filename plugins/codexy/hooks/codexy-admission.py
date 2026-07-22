#!/usr/bin/python3
"""Packaged, stateless Codex hook admission dispatcher."""

from __future__ import annotations

import argparse
import os
import signal
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from codexy_policy.admission import deny, evaluate


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument(
        "--event", required=True, choices=("PreToolUse", "PermissionRequest")
    )
    event = parser.parse_args().event
    signal.signal(signal.SIGALRM, lambda *_: _timeout(event))
    signal.alarm(3)
    output = evaluate(event, sys.stdin.buffer.read(1024 * 1024 + 1))
    signal.alarm(0)
    if output:
        sys.stdout.buffer.write(output)
    return 0


def _timeout(event: str) -> None:
    os.write(1, deny(event))
    os._exit(0)


if __name__ == "__main__":
    raise SystemExit(main())
