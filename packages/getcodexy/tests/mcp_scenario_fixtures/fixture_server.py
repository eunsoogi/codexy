"""Synthetic MCP server used by the scenario-core subprocess tests."""

from __future__ import annotations

import argparse
import json
import os
import select
import subprocess
import sys
import time
from pathlib import Path


SUPPORTED_PROTOCOL_VERSION = "2024-11-05"


def _send(identifier: int, value: dict) -> None:
    print(json.dumps({"jsonrpc": "2.0", "id": identifier, "result": value}), flush=True)


def _spawn_child(path: Path) -> None:
    child = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(30)"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    path.write_text(str(child.pid), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", required=True)
    parser.add_argument("--record", type=Path)
    parser.add_argument("--pid-file", type=Path)
    parser.add_argument("--literal")
    options = parser.parse_args()
    mode = options.mode
    initialized = False
    tools_listed = False
    if options.record:
        options.record.write_text(
            json.dumps(
                {
                    "argv": sys.argv[1:],
                    "cwd": os.getcwd(),
                    "marker": os.environ.get("SCENARIO_MARKER"),
                    "secret": os.environ.get("SCENARIO_SECRET"),
                    "literal": options.literal,
                }
            ),
            encoding="utf-8",
        )

    for line in sys.stdin:
        request = json.loads(line)
        identifier = request.get("id")
        method = request.get("method")
        if identifier is None:
            continue
        if method == "initialize":
            if mode == "delayed-init":
                time.sleep(0.2)
                if os.name != "nt" and select.select([sys.stdin], [], [], 0)[0]:
                    _send(
                        identifier,
                        {
                            "error": {
                                "code": -32004,
                                "message": "requests arrived before initialization",
                            }
                        },
                    )
                    continue
            _send(
                identifier,
                {
                    "protocolVersion": (
                        "2026-07-28"
                        if mode == "unsupported-version"
                        else SUPPORTED_PROTOCOL_VERSION
                    ),
                    "serverInfo": {"name": "synthetic-mcp", "version": "0.1"},
                    "capabilities": {"tools": {}},
                },
            )
            initialized = True
        elif method == "notifications/initialized":
            if not initialized:
                return 1
        elif method == "tools/list":
            if not initialized:
                print(
                    json.dumps(
                        {
                            "jsonrpc": "2.0",
                            "id": identifier,
                            "error": {"code": -32002, "message": "not initialized"},
                        }
                    ),
                    flush=True,
                )
                continue
            tools = [{"name": "read_value"}]
            if mode == "extra-tool":
                tools.append({"name": "forbidden_tool"})
            _send(identifier, {"tools": tools})
            tools_listed = True
        elif method == "tools/call":
            if not initialized or not tools_listed:
                print(
                    json.dumps(
                        {
                            "jsonrpc": "2.0",
                            "id": identifier,
                            "error": {"code": -32003, "message": "tools not ready"},
                        }
                    ),
                    flush=True,
                )
                continue
            if options.pid_file and mode in {"timeout", "cancel", "success-tree"}:
                _spawn_child(options.pid_file)
            if mode in {"timeout", "cancel", "success-tree"}:
                if mode == "success-tree":
                    _send(identifier, {"content": [{"type": "text", "text": "done"}]})
                    time.sleep(30)
                else:
                    time.sleep(30)
            elif mode == "json-rpc-error":
                print(
                    json.dumps(
                        {
                            "jsonrpc": "2.0",
                            "id": identifier,
                            "error": {"code": -32001, "message": "synthetic-secret"},
                        }
                    ),
                    flush=True,
                )
            elif mode == "tool-error":
                _send(
                    identifier,
                    {
                        "isError": True,
                        "content": [{"type": "text", "text": "synthetic-secret"}],
                    },
                )
            elif mode == "malformed":
                _send(identifier, {"isError": False})
            elif mode == "output-limit":
                print("synthetic-secret-" + ("x" * 20000), flush=True)
            else:
                arguments = request.get("params", {}).get("arguments", {})
                _send(
                    identifier,
                    {
                        "content": [{"type": "text", "text": "fixture-success"}],
                        "data": {
                            "public": "visible-value",
                            "secret": "synthetic-secret",
                            "arguments": arguments,
                        },
                    },
                )
                if mode == "success-tree":
                    time.sleep(30)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
