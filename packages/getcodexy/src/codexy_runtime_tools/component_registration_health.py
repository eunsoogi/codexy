"""Canonical ordinary-file registration checks used by the doctor report."""

from __future__ import annotations

import json
import os
import shutil
from pathlib import Path

from .component_core_hooks import (
    COMMAND_HOOKS as _CORE_COMMAND_HOOKS,
    DEPENDENCIES as CORE_HOOK_DEPENDENCIES,
    LAUNCHERS as CORE_HOOK_LAUNCHERS,
)
from .component_integrity import MAX_COMPONENT_BYTES, _read_regular, valid_agent_toml
from .component_mcp_materialization import (
    mcp_configuration,
    valid_component_mcp,
)
from .component_manifest import load_component_manifest

CATALOGS = {
    "core": """# Codexy packaged-agent discovery/registration contract. Validators and the
# registration script load agent_files from this catalog; native Codex agent
# use is through marker-owned standalone files under the Codex home agents directory.
version = "0.1.0"
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
  "codexy-watcher.toml",
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
        "codexy-watcher.toml",
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


def _bash_hooks(event: str) -> list[dict[str, object]]:
    return [_command_hook("^Bash$", "codexy-destructive-command", event)]


def _title_hooks(event: str) -> list[dict[str, object]]:
    return [
        _title_hook(
            "^(?:mcp__codex_apps__github_(?:create|update)_issue|github\\.(?:create|update)_issue)$",
            "issue",
            event,
        ),
        _title_hook(
            "^(?:mcp__codex_apps__github_(?:create|update)_pull_request|github\\.(?:create|update)_pull_request)$",
            "pr",
            event,
        ),
        _title_hook(
            "^(?:mcp__codex_apps__github_(?:merge_pull_request|enable_auto_merge)|github\\.(?:merge_pull_request|enable_auto_merge))$",
            "merge",
            event,
        ),
        _title_hook("^functions\\.exec$", "nested", event),
        _title_hook("^Bash$", "shell", event),
    ]


def _title_hook(matcher: str, kind: str, event: str) -> dict[str, object]:
    return {
        "matcher": matcher,
        "hooks": [
            {
                "type": "command",
                "command": f'"${{PLUGIN_ROOT}}/hooks/codexy-title-check.sh" {event} {kind}',
                "commandWindows": f'"${{PLUGIN_ROOT}}/hooks/codexy-title-check.cmd" {event} {kind}',
                "timeout": 5,
            }
        ],
    }


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
                *_title_hooks("PermissionRequest"),
                *_bash_hooks("PermissionRequest"),
            ],
            "PreToolUse": [
                *_title_hooks("PreToolUse"),
                *_bash_hooks("PreToolUse"),
            ],
        }
    },
}
MCP = mcp_configuration("devtools")
CORE_MCP = mcp_configuration("core")
LAUNCHERS = {
    "core": CORE_HOOK_LAUNCHERS,
    "github": (
        "hooks/codexy-github-workflow-context.sh",
        "hooks/codexy-github-workflow-context.cmd",
        "hooks/codexy-title-check.sh",
        "hooks/codexy-title-check.cmd",
        "hooks/codexy-destructive-command.sh",
        "hooks/codexy-destructive-command.cmd",
    ),
    "devtools": ("mcp/codexy-mcp-devtools",),
}


def valid_registration(plugin: Path, component: str) -> bool:
    """Require exactly the packaged registration and its local launch targets."""
    try:
        if component == "devtools":
            return valid_component_mcp(
                plugin, component, load_component_manifest().version
            )
        core_mcp = component == "core" and valid_component_mcp(
            plugin, component, load_component_manifest().version
        )
        return (
            (component != "core" or core_mcp)
            and _text(plugin / "agents/catalog.toml", plugin) == CATALOGS[component]
            and json.loads(_text(plugin / "hooks/hooks.json", plugin))
            == HOOKS[component]
            and all(
                _regular(plugin / f"agents/{name}", plugin, 'model = "')
                for name in AGENT_FILES[component]
            )
            and all(_launcher(plugin / path, plugin) for path in LAUNCHERS[component])
            and _skill(plugin, component)
        )
    except (KeyError, OSError, UnicodeDecodeError, SyntaxError, ValueError):
        return False


def _text(path: Path, root: Path) -> str:
    return _read_regular(root, path.relative_to(root), MAX_COMPONENT_BYTES).decode()


def _regular(path: Path, root: Path, needle: str = "") -> bool:
    contents = _read_regular(root, path.relative_to(root), MAX_COMPONENT_BYTES)
    return bool(contents) and (not needle or valid_agent_toml(contents.decode(), path))


def _launcher(path: Path, root: Path) -> bool:
    contents = _text(path, root) if _regular(path, root) else ""
    if path.suffix == ".cmd":
        return contents.lower().startswith("@echo off")
    command = contents.splitlines()[0][2:].split() if contents.startswith("#!") else []
    return (
        bool(command)
        and _executable(path, root)
        and (os.name == "nt" or shutil.which(command[-1]) is not None)
    )


def _skill(plugin: Path, component: str) -> bool:
    required = load_component_manifest().component(component).asset.required_paths
    if component == "core":
        required += CORE_HOOK_DEPENDENCIES
    name = "wiki" if component == "core" else "git-workflow"
    contents = _text(plugin / f"skills/{name}/SKILL.md", plugin)
    return all(_regular(plugin / path, plugin) for path in required) and (
        contents.startswith(f"---\nname: {name}\n") and "\n---\n" in contents
    )


def _executable(path: Path, root: Path) -> bool:
    return _regular(path, root) and (os.name == "nt" or os.access(path, os.X_OK))
