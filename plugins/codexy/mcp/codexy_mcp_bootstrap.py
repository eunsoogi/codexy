#!/usr/bin/env python3
"""Launch one Codexy MCP server using the plugin's selected release metadata."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path


REPOSITORY = "https://github.com/eunsoogi/codexy"
SERVERS = frozenset({"watcher", "lsp", "codegraph"})
SEMVER = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")


def main(arguments: list[str] | None = None) -> int:
    values = list(sys.argv[1:] if arguments is None else arguments)
    if not values or values[0] not in SERVERS:
        print(
            "codexy_mcp_bootstrap requires watcher, lsp, or codegraph",
            file=sys.stderr,
        )
        return 64
    server, server_arguments = values[0], values[1:]
    plugin_root = Path.cwd().resolve()
    try:
        version = _selected_version(plugin_root)
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
        print(
            f"codexy_mcp_bootstrap cannot read selected release: {error}",
            file=sys.stderr,
        )
        return 127
    uvx = shutil.which("uvx")
    if uvx is None:
        print(
            "codexy_mcp_bootstrap requires uvx on PATH; install uv",
            file=sys.stderr,
        )
        return 127
    command = [
        uvx,
        "--from",
        f"getcodexy=={version}",
        "codexy-mcp-runtime",
        server,
        "--plugin-root",
        str(plugin_root),
        "--",
        *server_arguments,
    ]
    environment = os.environ.copy()
    environment["CODEXY_PLUGIN_ROOT"] = str(plugin_root)
    try:
        if os.name == "nt":
            return subprocess.run(command, env=environment, check=False).returncode
        os.execvpe(uvx, command, environment)
    except OSError as error:
        print(f"codexy_mcp_bootstrap could not start uvx: {error}", file=sys.stderr)
        return 127
    return 127


def _selected_version(plugin_root: Path) -> str:
    manifest = json.loads(
        (plugin_root / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
    )
    if not isinstance(manifest, dict) or (
        manifest.get("name") not in {"codexy", "codexy-devtools"}
        or manifest.get("repository") != REPOSITORY
    ):
        raise ValueError("plugin manifest identity is not Codexy")
    version = manifest.get("version")
    if not isinstance(version, str) or SEMVER.fullmatch(version) is None:
        raise ValueError("plugin manifest version is not a stable semantic version")
    return version


if __name__ == "__main__":
    raise SystemExit(main())
