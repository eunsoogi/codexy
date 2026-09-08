"""Small local Watcher runtime used by installed-component tests."""

from __future__ import annotations

import json
from pathlib import Path


FAKE_WATCHER = r'''#!/usr/bin/env python3
import json
import sys

for line in sys.stdin:
    request = json.loads(line)
    identifier = request.get("id")
    if identifier is None:
        continue
    method = request.get("method")
    if method == "initialize":
        result = {"serverInfo": {"name": "codexy-watcher", "version": "fixture"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "watcher_health"}]}
    elif method == "tools/call":
        name = request.get("params", {}).get("name")
        if name == "watcher_open":
            payload = {"sessionId": "fixture-session", "parentToken": "fixture-token"}
            result = {"content": [{"type": "text", "text": json.dumps(payload)}]}
        else:
            result = {"content": [{"type": "text", "text": "ok"}]}
    else:
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": identifier, "result": result}), flush=True)
'''


def install_watcher_runtime(plugin: Path) -> None:
    runtime = plugin / "runtime"
    runtime.mkdir(parents=True, exist_ok=True)
    for platform in ("darwin-arm64", "linux-x86_64"):
        path = runtime / f"codexy-mcp-watcher-{platform}.bin"
        path.write_text(FAKE_WATCHER, encoding="utf-8")
        path.chmod(0o755)
