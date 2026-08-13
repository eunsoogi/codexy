"""Read-only validation of the managed plugin registration surfaces."""

from __future__ import annotations

import json
import os
import stat
import tomllib
from pathlib import Path


CATALOGS = {
    "core": {
        "version": "0.1.0",
        "default_branch_prefix": "codexy/",
        "catalog_kind": "plugin-packaged-specialist-agent-files",
        "native_custom_agent_registration": "codex-home-standalone-agent-projection",
        "native_custom_agent_projection": "managed-codexy-subdirectory",
        "agent_files": ["codexy-architect.toml", "codexy-cartographer.toml", "codexy-auditor.toml", "codexy-shipwright.toml", "codexy-inspector.toml", "codexy-sentinel.toml", "codexy-warden.toml"],
    },
    "github": {"version": "0.1.0", "catalog_kind": "plugin-packaged-specialist-agent-files", "agent_files": ["codexy-weaver.toml"]},
}
HOOKS = {
    "core": {
        "hooks": {
            "PermissionRequest": [{"matcher": "^codex_app__send_message_to_thread$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PermissionRequest", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PermissionRequest", "timeout": 5}]}],
            "PreToolUse": [{"matcher": "^codex_app__send_message_to_thread$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PreToolUse", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PreToolUse", "timeout": 5}]}],
        }
    },
    "github": {
        "hooks": {
            "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh\"", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.cmd\"", "timeout": 5}]}],
            "PreToolUse": [
                {"matcher": "^mcp__codex_apps__github_(create|update)_issue$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule issue", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-issue.cmd\"", "timeout": 5}]},
                {"matcher": "^mcp__codex_apps__github_create_pull_request$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule pr", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-pr.cmd\"", "timeout": 5}]},
            ],
        }
    },
}
MCP = {
    "lsp": {"command": "./mcp/codexy-mcp-devtools", "args": ["lsp", "--stdio"], "cwd": "."},
    "codegraph": {"command": "./mcp/codexy-mcp-devtools", "args": ["codegraph", "--stdio"], "cwd": "."},
}
SURFACE_PATHS = {
    "core": ("agents/catalog.toml", "hooks/hooks.json", "hooks/codexy-thread-delivery.sh", "hooks/codexy-thread-delivery.cmd"),
    "github": ("agents/catalog.toml", "hooks/hooks.json", "hooks/codexy-github-workflow-context.sh", "hooks/codexy-github-workflow-context.cmd", "hooks/codexy-github-admission.sh", "hooks/codexy-github-admission-issue.cmd", "hooks/codexy-github-admission-pr.cmd"),
    "devtools": ("mcp/codexy-mcp-devtools",),
}


def valid_surface(plugin: Path, component: str) -> bool:
    if any(not _regular(plugin / path) for path in SURFACE_PATHS[component]):
        return False
    if component == "devtools":
        return os.access(plugin / "mcp/codexy-mcp-devtools", os.X_OK) and _json_value(plugin / ".mcp.json") == MCP
    catalog = _toml_value(plugin / "agents/catalog.toml")
    if catalog != CATALOGS[component] or any(not _regular(plugin / "agents" / name) for name in CATALOGS[component]["agent_files"]):
        return False
    return _json_value(plugin / "hooks/hooks.json") == HOOKS[component]


def _regular(path: Path) -> bool:
    try:
        return path.is_file() and not path.is_symlink()
    except OSError:
        return False


def _json_value(path: Path) -> object | None:
    try:
        return json.loads(_contents(path).decode(), object_pairs_hook=_unique_object)
    except (OSError, UnicodeDecodeError, ValueError):
        return None


def _toml_value(path: Path) -> object | None:
    try:
        return tomllib.loads(_contents(path).decode())
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError):
        return None


def _contents(path: Path) -> bytes:
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise OSError("registration is not a regular file")
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
            raise OSError("registration changed while reading")
        return os.read(descriptor, opened.st_size)
    finally:
        os.close(descriptor)


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("registration has duplicate keys")
        result[key] = value
    return result
