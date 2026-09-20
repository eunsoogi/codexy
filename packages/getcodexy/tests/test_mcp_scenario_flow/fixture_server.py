"""Synthetic MCP server for ordered scenario-flow tests."""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path


PROTOCOL_VERSION = "2024-11-05"


def _send(identifier: int, result: dict) -> None:
    print(
        json.dumps({"jsonrpc": "2.0", "id": identifier, "result": result}), flush=True
    )


def _record(path: Path | None, arguments: dict) -> None:
    if path is None:
        return
    path.write_text(
        json.dumps(
            {
                "argv": sys.argv[1:],
                "cwd": os.getcwd(),
                "marker": os.environ.get("SCENARIO_MARKER"),
                "arguments": arguments,
            }
        ),
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", required=True)
    parser.add_argument("--record", type=Path)
    parser.add_argument("--value")
    options = parser.parse_args()
    initialized = False
    listed = False

    for line in sys.stdin:
        request = json.loads(line)
        identifier = request.get("id")
        method = request.get("method")
        if identifier is None:
            continue
        if method == "initialize":
            _send(
                identifier,
                {
                    "protocolVersion": PROTOCOL_VERSION,
                    "serverInfo": {"name": "flow-fixture", "version": "0.1"},
                },
            )
            initialized = True
        elif method == "notifications/initialized":
            if not initialized:
                return 1
        elif method == "tools/list":
            _send(identifier, {"tools": [{"name": "read_value"}]})
            listed = True
        elif method == "tools/call":
            if not initialized or not listed:
                return 1
            arguments = request.get("params", {}).get("arguments", {})
            _record(options.record, arguments)
            if options.mode == "slow":
                time.sleep(30)
            elif options.mode == "malformed":
                _send(identifier, {"unexpected": True})
            elif options.mode == "expected-error":
                print(
                    json.dumps(
                        {
                            "jsonrpc": "2.0",
                            "id": identifier,
                            "error": {"code": -32020, "message": "expected"},
                        }
                    ),
                    flush=True,
                )
            elif options.mode == "search":
                _send(
                    identifier,
                    {
                        "content": [{"type": "text", "text": "search-complete"}],
                        "data": {"id": "item-42", "status": "ready"},
                    },
                )
            elif options.mode == "missing-id":
                _send(
                    identifier,
                    {
                        "content": [{"type": "text", "text": "search-complete"}],
                        "data": {"status": "ready"},
                    },
                )
            elif options.mode == "wrong-type":
                _send(
                    identifier,
                    {
                        "content": [{"type": "text", "text": "search-complete"}],
                        "data": {"id": 42},
                    },
                )
            elif options.mode == "wrong-value":
                _send(
                    identifier,
                    {
                        "content": [{"type": "text", "text": "search-complete"}],
                        "data": {"id": "wrong-id"},
                    },
                )
            elif options.mode == "malicious":
                _send(
                    identifier,
                    {
                        "content": [{"type": "text", "text": "search-complete"}],
                        "data": {"id": options.value},
                    },
                )
            else:
                _send(
                    identifier,
                    {
                        "content": [{"type": "text", "text": "detail-complete"}],
                        "data": {"matched_id": arguments.get("id")},
                    },
                )
            return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
