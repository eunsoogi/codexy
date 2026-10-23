"""Small Watcher runtime fixture for the verified legacy-upgrade package."""

from __future__ import annotations

import json
import os
import shutil
import sys
from pathlib import Path

from codexy_runtime_tools.component_mcp_materialization import materialize_component_mcp

LEGACY_VERSION = "1.13.4"
LEGACY_MARKETPLACE_ENV = "CODEXY_TEST_LEGACY_MARKETPLACE_ROOT"


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


def verified_legacy_marketplace_root() -> Path | None:
    value = os.environ.get(LEGACY_MARKETPLACE_ENV)
    if not value:
        if os.name == "nt" and os.environ.get("CI", "").lower() == "true":
            raise RuntimeError(
                "verified 1.13.4 upgrade bundle is required in Windows CI"
            )
        return None
    root = Path(value).resolve()
    receipt_path = root.parent / "download/runtime-release-receipt.json"
    if not root.is_dir() or not receipt_path.is_file():
        raise RuntimeError(f"verified legacy marketplace bundle is incomplete: {root}")
    try:
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(
            f"verified legacy release receipt is unreadable: {error}"
        ) from error
    if not isinstance(receipt, dict):
        raise RuntimeError("verified legacy release receipt must be an object")
    release = receipt.get("release")
    if not isinstance(release, dict) or release.get("tag") != f"v{LEGACY_VERSION}":
        raise RuntimeError(
            "legacy marketplace fixture is not the verified 1.13.4 release"
        )
    for plugin in ("codexy", "codexy-github", "codexy-devtools"):
        manifest_path = root / "plugins" / plugin / ".codex-plugin/plugin.json"
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise RuntimeError(
                f"verified legacy manifest is unreadable: {manifest_path}"
            ) from error
        if manifest.get("name") != plugin or manifest.get("version") != LEGACY_VERSION:
            raise RuntimeError(f"verified legacy package identity mismatch: {plugin}")
    watcher = root / "plugins/codexy/mcp/codexy-mcp-watcher.exe"
    if not watcher.is_file() or watcher.read_bytes()[:2] != b"MZ":
        raise RuntimeError(
            "verified 1.13.4 package is missing its native Windows Watcher"
        )
    return root


def install_watcher_runtime(
    plugin: Path, *, verified_legacy_upgrade_fixture: bool = False
) -> None:
    if not verified_legacy_upgrade_fixture:
        raise RuntimeError(
            "Watcher test runtime requires the explicit legacy upgrade fixture"
        )
    manifest = json.loads(
        (plugin / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
    )
    version = manifest.get("version")
    if version != LEGACY_VERSION:
        raise RuntimeError(
            "Watcher test runtime is restricted to the 1.13.4 upgrade fixture"
        )
    legacy_root = verified_legacy_marketplace_root()
    if legacy_root is None:
        raise RuntimeError(
            "verified 1.13.4 upgrade bundle is required for Watcher fixtures"
        )
    runtime = plugin / "runtime"
    runtime.mkdir(parents=True, exist_ok=True)
    for platform in ("darwin-arm64", "linux-x86_64"):
        path = runtime / f"codexy-mcp-watcher-{platform}.bin"
        path.write_text(FAKE_WATCHER, encoding="utf-8")
        path.chmod(0o755)
    if os.name == "nt":
        binary = runtime / "codexy-mcp-watcher-windows-x86_64.exe"
        source = legacy_root / "plugins/codexy/mcp/codexy-mcp-watcher.exe"
        shutil.copyfile(source, binary)
        binary.chmod(0o755)
        script = runtime / "codexy-mcp-watcher-windows-x86_64.py"
        script.write_text(FAKE_WATCHER, encoding="utf-8")
        (plugin / "mcp/codexy-mcp-watcher.cmd").write_text(
            f'@echo off\r\n"{sys.executable}" "%~dp0..\\runtime\\{script.name}" %*\r\n'
            "exit /b %ERRORLEVEL%\r\n",
            encoding="utf-8",
        )
    # Validate against the copied package version so retained launchers do not
    # recreate MCP registration for the intentional no-MCP core package.
    materialize_component_mcp(plugin, "core", version)
