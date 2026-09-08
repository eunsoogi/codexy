#!/usr/bin/env python3
"""Interactively smoke the POSIX native MCP runtime binaries."""

import json
import os
from pathlib import Path
import select
import subprocess
import tempfile


CASES = (
    (
        "packages/codexy-runtime/target/release/codexy-mcp-lsp",
        "lsp_status",
        {"root": ".", "path": "packages/codexy-runtime/Cargo.toml"},
    ),
    (
        "packages/codexy-runtime/target/release/codexy-mcp-codegraph",
        "codegraph_overview",
        {"root": ".", "limit": 10},
    ),
    (
        "packages/codexy-runtime/target/release/codexy-mcp-watcher",
        "watcher_open",
        {
            "assignmentId": "runtime-smoke",
            "parent": {"id": "smoke"},
            "watcher": {"id": "smoke"},
            "targets": [{"threadId": "smoke"}],
            "ttlSeconds": 1,
        },
    ),
)


def request(process, payload):
    process.stdin.write(json.dumps(payload) + "\n")
    process.stdin.flush()
    ready, _, _ = select.select([process.stdout], [], [], 30)
    if not ready:
        raise RuntimeError("MCP response timed out")
    line = process.stdout.readline()
    if not line:
        raise RuntimeError("MCP process closed stdout before responding")
    return json.loads(line)


def smoke_case(binary, name, arguments, state_dir):
    process = subprocess.Popen(
        [binary],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        env={**os.environ, "CODEXY_WATCHER_STATE_DIR": str(state_dir)},
    )
    try:
        initialize = request(
            process,
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
        )
        tool = request(
            process,
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments},
            },
        )
        if initialize.get("result", {}).get("protocolVersion") != "2024-11-05":
            raise RuntimeError(f"{binary} protocol mismatch")
        if tool.get("error") or not tool.get("result", {}).get("content"):
            raise RuntimeError(f"{binary} tool smoke failed: {tool}")
    finally:
        process.stdin.close()
        try:
            process.wait(timeout=30)
        except subprocess.TimeoutExpired as error:
            process.kill()
            process.wait()
            raise RuntimeError(f"{binary} did not exit after stdin close") from error
        stderr = process.stderr.read()
        if process.returncode:
            raise RuntimeError(f"{binary} failed: {stderr}")


def main():
    root = Path.cwd()
    with tempfile.TemporaryDirectory(prefix="codexy-mcp-smoke-") as state:
        state_dir = Path(state)
        for relative, name, arguments in CASES:
            smoke_case(str(root / relative), name, arguments, state_dir)


if __name__ == "__main__":
    main()
