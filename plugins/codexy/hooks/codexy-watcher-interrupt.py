#!/usr/bin/python3
"""Bind Watcher waits to the host turn and release them on Interrupt."""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

MAX_INPUT_BYTES = 1024 * 1024
EVENTS = ("PreToolUse", "Interrupt")


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--event", required=True, choices=EVENTS)
    event = parser.parse_args().event
    raw = sys.stdin.buffer.read(MAX_INPUT_BYTES + 1)
    if len(raw) > MAX_INPUT_BYTES:
        return 0
    try:
        payload = json.loads(raw)
    except (TypeError, ValueError, json.JSONDecodeError):
        return 0
    if not isinstance(payload, dict) or payload.get("hook_event_name") != event:
        return 0
    if event == "Interrupt":
        _invoke("--hook-interrupt", payload)
        return 0
    tool_name = payload.get("tool_name")
    tool_input = payload.get("tool_input")
    if not isinstance(tool_name, str) or (
        tool_name != "watcher_wait" and not tool_name.endswith("__watcher_wait")
    ):
        return 0
    if not isinstance(tool_input, dict):
        return 0
    result = _invoke("--hook-pretool", payload)
    binding = result.get("requestBinding") if isinstance(result, dict) else None
    if not isinstance(binding, str) or not binding:
        return 0
    updated = dict(tool_input)
    updated["requestBinding"] = binding
    output = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "updatedInput": updated,
        }
    }
    sys.stdout.write(json.dumps(output, separators=(",", ":")))
    return 0


def _invoke(argument: str, payload: dict[str, object]) -> object:
    runtime = _runtime()
    if runtime is None:
        return {}
    try:
        result = subprocess.run(
            [str(runtime), argument],
            input=json.dumps(payload, separators=(",", ":")).encode(),
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=2,
            env=os.environ.copy(),
        )
    except (OSError, subprocess.SubprocessError):
        return {}
    if result.returncode != 0:
        return {}
    try:
        return json.loads(result.stdout)
    except (TypeError, ValueError, json.JSONDecodeError):
        return {}


def _runtime() -> Path | None:
    root = Path(os.environ.get("PLUGIN_ROOT", Path(__file__).resolve().parents[1]))
    name, extension = _runtime_name()
    candidates = []
    configured = os.environ.get("CODEXY_RUNTIME_DIR")
    if configured and Path(configured).is_absolute():
        candidates.append(Path(configured) / f"{name}{extension}")
    candidates.append(root / "runtime" / f"{name}{extension}")
    return next(
        (candidate for candidate in candidates if candidate.is_file() and os.access(candidate, os.X_OK)),
        None,
    )


def _runtime_name() -> tuple[str, str]:
    if os.name == "nt":
        return "codexy-mcp-watcher-windows-x86_64", ".exe"
    if sys.platform == "darwin":
        return "codexy-mcp-watcher-darwin-arm64", ".bin"
    return "codexy-mcp-watcher-linux-x86_64", ".bin"


if __name__ == "__main__":
    raise SystemExit(main())
