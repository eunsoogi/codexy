"""Canonical ordinary-file registration checks used by the doctor report."""

from __future__ import annotations

import json
import os
import tomllib
from pathlib import Path


CATALOGS = {
    "core": {
        "version": "0.1.0", "default_branch_prefix": "codexy/", "catalog_kind": "plugin-packaged-specialist-agent-files",
        "native_custom_agent_registration": "codex-home-standalone-agent-projection", "native_custom_agent_projection": "managed-codexy-subdirectory",
        "agent_files": ["codexy-architect.toml", "codexy-cartographer.toml", "codexy-auditor.toml", "codexy-shipwright.toml", "codexy-inspector.toml", "codexy-sentinel.toml", "codexy-warden.toml"],
    },
    "github": {"version": "0.1.0", "catalog_kind": "plugin-packaged-specialist-agent-files", "agent_files": ["codexy-weaver.toml"]},
}
HOOKS = {
    "core": {"hooks": {
        "PermissionRequest": [{"matcher": "^codex_app__send_message_to_thread$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PermissionRequest", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PermissionRequest", "timeout": 5}]}],
        "PreToolUse": [{"matcher": "^codex_app__send_message_to_thread$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PreToolUse", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PreToolUse", "timeout": 5}]}],
    }},
    "github": {"hooks": {
        "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh\"", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.cmd\"", "timeout": 5}]}],
        "PreToolUse": [
            {"matcher": "^mcp__codex_apps__github_(create|update)_issue$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule issue", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-issue.cmd\"", "timeout": 5}]},
            {"matcher": "^mcp__codex_apps__github_create_pull_request$", "hooks": [{"type": "command", "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule pr", "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-pr.cmd\"", "timeout": 5}]},
        ],
    }},
}
MCP = {
    "lsp": {"command": "./mcp/codexy-mcp-devtools", "args": ["lsp", "--stdio"], "cwd": "."},
    "codegraph": {"command": "./mcp/codexy-mcp-devtools", "args": ["codegraph", "--stdio"], "cwd": "."},
}
LAUNCHERS = {
    "core": ("hooks/codexy-thread-delivery.sh", "hooks/codexy-thread-delivery.cmd"),
    "github": ("hooks/codexy-github-workflow-context.sh", "hooks/codexy-github-workflow-context.cmd", "hooks/codexy-github-admission.sh", "hooks/codexy-github-admission-issue.cmd", "hooks/codexy-github-admission-pr.cmd"),
    "devtools": ("mcp/codexy-mcp-devtools",),
}


def valid_registration(plugin: Path, component: str) -> bool:
    """Require exactly the packaged registration and its local launch targets."""
    try:
        if component == "devtools":
            return _json(plugin / ".mcp.json") == MCP and _executable(plugin / LAUNCHERS[component][0])
        catalog = _toml(plugin / "agents/catalog.toml")
        agents = catalog.get("agent_files") if isinstance(catalog, dict) else ()
        return catalog == CATALOGS[component] and _json(plugin / "hooks/hooks.json") == HOOKS[component] and all(_regular(plugin / f"agents/{name}") for name in agents) and all(_regular(plugin / path) for path in LAUNCHERS[component])
    except (KeyError, OSError, UnicodeDecodeError, ValueError, tomllib.TOMLDecodeError):
        return False


def _json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as source:
        return json.load(source)


def _toml(path: Path) -> object:
    with path.open("rb") as source:
        return tomllib.load(source)


def _regular(path: Path) -> bool:
    try:
        return path.is_file() and not path.is_symlink()
    except OSError:
        return False


def _executable(path: Path) -> bool:
    return _regular(path) and os.access(path, os.X_OK)
