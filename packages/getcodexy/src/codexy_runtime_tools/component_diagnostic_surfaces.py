"""Read-only validation of managed registrations through admitted no-follow trees."""

from __future__ import annotations

import json
import tomllib

from .component_source_admission import DiagnosticTree


CATALOGS = {
    "core": {
        "version": "0.1.0",
        "default_branch_prefix": "codexy/",
        "catalog_kind": "plugin-packaged-specialist-agent-files",
        "native_custom_agent_registration": "codex-home-standalone-agent-projection",
        "native_custom_agent_projection": "managed-codexy-subdirectory",
        "agent_files": [
            "codexy-architect.toml",
            "codexy-cartographer.toml",
            "codexy-auditor.toml",
            "codexy-shipwright.toml",
            "codexy-inspector.toml",
            "codexy-sentinel.toml",
            "codexy-warden.toml",
        ],
    },
    "github": {
        "version": "0.1.0",
        "catalog_kind": "plugin-packaged-specialist-agent-files",
        "agent_files": ["codexy-weaver.toml"],
    },
}
HOOKS = {
    "core": {
        "hooks": {
            "PermissionRequest": [{
                "matcher": "^codex_app__send_message_to_thread$",
                "hooks": [{
                    "type": "command",
                    "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PermissionRequest",
                    "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PermissionRequest",
                    "timeout": 5,
                }],
            }],
            "PreToolUse": [{
                "matcher": "^codex_app__send_message_to_thread$",
                "hooks": [{
                    "type": "command",
                    "command": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.sh\" PreToolUse",
                    "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-thread-delivery.cmd\" PreToolUse",
                    "timeout": 5,
                }],
            }],
        },
    },
    "github": {
        "hooks": {
            "UserPromptSubmit": [{"hooks": [{
                "type": "command",
                "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh\"",
                "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.cmd\"",
                "timeout": 5,
            }]}],
            "PreToolUse": [
                {
                    "matcher": "^mcp__codex_apps__github_(create|update)_issue$",
                    "hooks": [{
                        "type": "command",
                        "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule issue",
                        "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-issue.cmd\"",
                        "timeout": 5,
                    }],
                },
                {
                    "matcher": "^mcp__codex_apps__github_create_pull_request$",
                    "hooks": [{
                        "type": "command",
                        "command": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh\" --rule pr",
                        "commandWindows": "\"${PLUGIN_ROOT}/hooks/codexy-github-admission-pr.cmd\"",
                        "timeout": 5,
                    }],
                },
            ],
        },
    },
}
MCP = {
    "lsp": {"command": "./mcp/codexy-mcp-devtools", "args": ["lsp", "--stdio"], "cwd": "."},
    "codegraph": {"command": "./mcp/codexy-mcp-devtools", "args": ["codegraph", "--stdio"], "cwd": "."},
}
SURFACE_PATHS = {
    "core": ("agents/catalog.toml", "hooks/hooks.json", "hooks/codexy-thread-delivery.sh", "hooks/codexy-thread-delivery.cmd"),
    "github": (
        "agents/catalog.toml",
        "hooks/hooks.json",
        "hooks/codexy-github-workflow-context.sh",
        "hooks/codexy-github-workflow-context.cmd",
        "hooks/codexy-github-admission.sh",
        "hooks/codexy-github-admission-issue.cmd",
        "hooks/codexy-github-admission-pr.cmd",
    ),
    "devtools": ("mcp/codexy-mcp-devtools", ".mcp.json"),
}


def valid_surface(tree: DiagnosticTree, component: str) -> bool:
    if any(tree.read_regular(path) is None for path in SURFACE_PATHS[component]):
        return False
    if component == "devtools":
        return tree.executable("mcp/codexy-mcp-devtools") and _json_value(tree, ".mcp.json") == MCP
    catalog = _toml_value(tree, "agents/catalog.toml")
    if catalog != CATALOGS[component]:
        return False
    if any(tree.read_regular(f"agents/{name}") is None for name in CATALOGS[component]["agent_files"]):
        return False
    return _json_value(tree, "hooks/hooks.json") == HOOKS[component]


def _json_value(tree: DiagnosticTree, relative: str) -> object | None:
    contents = tree.read_regular(relative)
    try:
        return json.loads(contents.decode(), object_pairs_hook=_unique_object) if contents is not None else None
    except (UnicodeDecodeError, ValueError):
        return None


def _toml_value(tree: DiagnosticTree, relative: str) -> object | None:
    contents = tree.read_regular(relative)
    try:
        return tomllib.loads(contents.decode()) if contents is not None else None
    except (UnicodeDecodeError, tomllib.TOMLDecodeError):
        return None


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("registration has duplicate keys")
        result[key] = value
    return result
