"""Canonical ordinary-file registration checks used by the doctor report."""

from __future__ import annotations

import json
import os
from pathlib import Path

from .component_integrity import verify_component


CATALOGS = {
    "core": """# Codexy packaged-agent discovery/registration contract. Validators and the
# registration script load agent_files from this catalog; native Codex agent
# use is through marker-owned standalone files under the Codex home agents directory.
version = "0.1.0"
default_branch_prefix = "codexy/"
catalog_kind = "plugin-packaged-specialist-agent-files"
native_custom_agent_registration = "codex-home-standalone-agent-projection"
native_custom_agent_projection = "managed-codexy-subdirectory"
agent_files = [
  "codexy-architect.toml",
  "codexy-cartographer.toml",
  "codexy-auditor.toml",
  "codexy-shipwright.toml",
  "codexy-inspector.toml",
  "codexy-sentinel.toml",
  "codexy-warden.toml",
]
""",
    "github": """version = "0.1.0"
catalog_kind = "plugin-packaged-specialist-agent-files"
agent_files = ["codexy-weaver.toml"]
""",
}
AGENT_FILES = {
    "core": (
        "codexy-architect.toml",
        "codexy-cartographer.toml",
        "codexy-auditor.toml",
        "codexy-shipwright.toml",
        "codexy-inspector.toml",
        "codexy-sentinel.toml",
        "codexy-warden.toml",
    ),
    "github": ("codexy-weaver.toml",),
}


def _command_hook(matcher: str, launcher_stem: str, event: str) -> dict[str, object]:
    return {
        "matcher": matcher,
        "hooks": [
            {
                "type": "command",
                "command": f'"${{PLUGIN_ROOT}}/hooks/{launcher_stem}.sh" {event}',
                "commandWindows": f'"${{PLUGIN_ROOT}}/hooks/{launcher_stem}.cmd" {event}',
                "timeout": 5,
            }
        ],
    }


_CORE_COMMAND_HOOKS = (
    ("^codex_app__send_message_to_thread$", "codexy-thread-delivery"),
    ("^codex_app__create_thread$", "codexy-child-thread-creation"),
)


HOOKS = {
    "core": {
        "hooks": {
            event: [
                _command_hook(matcher, launcher_stem, event)
                for matcher, launcher_stem in _CORE_COMMAND_HOOKS
            ]
            for event in ("PermissionRequest", "PreToolUse")
        }
    },
    "github": {
        "hooks": {
            "UserPromptSubmit": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.sh"',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context.cmd"',
                            "timeout": 5,
                        }
                    ]
                }
            ],
            "PermissionRequest": [
                {
                    "matcher": "^Bash$",
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-repository-github-command.sh" PermissionRequest',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-repository-github-command.cmd" PermissionRequest',
                            "timeout": 5,
                        }
                    ],
                },
                {
                    "matcher": "^Bash$",
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-destructive-command.sh" PermissionRequest',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-destructive-command.cmd" PermissionRequest',
                            "timeout": 5,
                        }
                    ],
                },
            ],
            "PreToolUse": [
                {
                    "matcher": "^mcp__codex_apps__github_(create|update)_issue$",
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh" --rule issue',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-github-admission-issue.cmd"',
                            "timeout": 5,
                        }
                    ],
                },
                {
                    "matcher": "^mcp__codex_apps__github_create_pull_request$",
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-github-admission.sh" --rule pr',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-github-admission-pr.cmd"',
                            "timeout": 5,
                        }
                    ],
                },
                {
                    "matcher": "^Bash$",
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-repository-github-command.sh" PreToolUse',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-repository-github-command.cmd" PreToolUse',
                            "timeout": 5,
                        }
                    ],
                },
                {
                    "matcher": "^Bash$",
                    "hooks": [
                        {
                            "type": "command",
                            "command": '"${PLUGIN_ROOT}/hooks/codexy-destructive-command.sh" PreToolUse',
                            "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-destructive-command.cmd" PreToolUse',
                            "timeout": 5,
                        }
                    ],
                },
            ],
        }
    },
}
MCP = {
    "lsp": {
        "command": "./mcp/codexy-mcp-devtools",
        "args": ["lsp", "--stdio"],
        "cwd": ".",
    },
    "codegraph": {
        "command": "./mcp/codexy-mcp-devtools",
        "args": ["codegraph", "--stdio"],
        "cwd": ".",
    },
}
LAUNCHERS = {
    "core": (
        "hooks/codexy-thread-delivery.sh",
        "hooks/codexy-thread-delivery.cmd",
        "hooks/codexy-child-thread-creation.sh",
        "hooks/codexy-child-thread-creation.cmd",
    ),
    "github": (
        "hooks/codexy-github-workflow-context.sh",
        "hooks/codexy-github-workflow-context.cmd",
        "hooks/codexy-github-admission.sh",
        "hooks/codexy-github-admission-issue.cmd",
        "hooks/codexy-github-admission-pr.cmd",
        "hooks/codexy-repository-github-command.sh",
        "hooks/codexy-repository-github-command.cmd",
        "hooks/codexy-destructive-command.sh",
        "hooks/codexy-destructive-command.cmd",
    ),
    "devtools": ("mcp/codexy-mcp-devtools",),
}
CORE_HOOK_DEPENDENCIES = (
    "hooks/codexy-child-thread-creation.py",
    "hooks/codexy_policy/child_thread_creation.py",
    "hooks/codexy_policy/envelope.py",
)


def valid_registration(plugin: Path, component: str) -> bool:
    """Require exactly the packaged registration and its local launch targets."""
    try:
        if component == "devtools":
            return _json(plugin / ".mcp.json") == MCP and _executable(
                plugin / LAUNCHERS[component][0]
            )
        verify_component(plugin, "codexy" if component == "core" else "codexy-github")
        return (
            _text(plugin / "agents/catalog.toml") == CATALOGS[component]
            and _json(plugin / "hooks/hooks.json") == HOOKS[component]
            and all(
                _regular(plugin / f"agents/{name}") for name in AGENT_FILES[component]
            )
            and all(_regular(plugin / path) for path in LAUNCHERS[component])
            and (
                component != "core"
                or all(_regular(plugin / path) for path in CORE_HOOK_DEPENDENCIES)
            )
        )
    except (KeyError, OSError, UnicodeDecodeError, ValueError):
        return False


def _json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as source:
        return json.load(source)


def _text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _regular(path: Path) -> bool:
    try:
        return path.is_file() and not path.is_symlink() and path.stat().st_size > 0
    except OSError:
        return False


def _executable(path: Path) -> bool:
    return _regular(path) and os.access(path, os.X_OK)
