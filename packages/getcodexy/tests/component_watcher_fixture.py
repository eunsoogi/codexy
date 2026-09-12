"""Small local Watcher runtime used by installed-component tests."""

from __future__ import annotations

import json
import os
import shutil
import sys
import unittest
from pathlib import Path

from codexy_runtime_tools.component_mcp_materialization import materialize_component_mcp


FAKE_WATCHER = r"""#!/usr/bin/env python3
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
"""


def require_native_watcher_binary() -> Path:
    native_binary = os.environ.get("CODEXY_TEST_WATCHER_BINARY")
    if native_binary:
        source = Path(native_binary)
        if not source.is_file():
            raise RuntimeError(f"missing hosted Windows Watcher runtime: {source}")
        return source
    if os.environ.get("CI", "").lower() == "true":
        raise RuntimeError("hosted Windows Watcher runtime is required in CI")
    raise unittest.SkipTest("hosted Windows Watcher runtime is required")


def install_watcher_runtime(plugin: Path) -> None:
    runtime = plugin / "runtime"
    runtime.mkdir(parents=True, exist_ok=True)
    for platform in ("darwin-arm64", "linux-x86_64"):
        path = runtime / f"codexy-mcp-watcher-{platform}.bin"
        path.write_text(FAKE_WATCHER, encoding="utf-8")
        path.chmod(0o755)
    if os.name == "nt":
        binary = runtime / "codexy-mcp-watcher-windows-x86_64.exe"
        source = require_native_watcher_binary()
        shutil.copyfile(source, binary)
        binary.chmod(0o755)
        script = runtime / "codexy-mcp-watcher-windows-x86_64.py"
        script.write_text(FAKE_WATCHER, encoding="utf-8")
        (plugin / "mcp/codexy-mcp-watcher.cmd").write_text(
            f'@echo off\r\n"{sys.executable}" "%~dp0..\\runtime\\{script.name}" %*\r\n'
            "exit /b %ERRORLEVEL%\r\n",
            encoding="utf-8",
        )
    materialize_component_mcp(plugin, "core")
