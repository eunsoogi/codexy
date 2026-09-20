"""Canonical ordinary-file registration checks used by the doctor report."""

from __future__ import annotations

import json
from pathlib import Path

from .component_core_hooks import (
    COMMAND_HOOKS as _CORE_COMMAND_HOOKS,
    DEPENDENCIES as CORE_HOOK_DEPENDENCIES,
    LIFECYCLE_HOOKS as _CORE_LIFECYCLE_HOOKS,
    LAUNCHERS as CORE_HOOK_LAUNCHERS,
)
from .component_mcp_materialization import (
    mcp_configuration,
    valid_component_mcp,
)
from .component_manifest import load_component_manifest
from .component_registration_catalog import CATALOGS, _catalog_agent_files
from .component_registration_files import _launcher, _regular, _skill, _text

MANAGED_MARKERS = {
    "core": "# CODEXY MANAGED AGENT\n",
    "github": "# Managed by Codexy GitHub.\n",
}
MANAGED_ROOTS = {
    "core": Path("agents/codexy"),
    "github": Path("agents/codexy-github"),
}
REGISTRATION_SOURCE = "codex-home-standalone-agent-projection"
REGISTRATION_SCOPE = "current-doctor-invocation"
REGISTRATION_REPAIR = "repair the Codexy registration, then rerun getcodexy doctor"


def _hook(command: str, windows: str, timeout: int) -> dict[str, object]:
    return {
        "type": "command",
        "command": command,
        "commandWindows": windows,
        "timeout": timeout,
    }


def _command_hook(matcher: str, launcher_stem: str, event: str) -> dict[str, object]:
    command = f'"${{PLUGIN_ROOT}}/hooks/{launcher_stem}'
    return {
        "matcher": matcher,
        "hooks": [_hook(f'{command}.sh" {event}', f'{command}.cmd" {event}', 5)],
    }


def _lifecycle_hook(launcher_stem: str, event: str) -> dict[str, object]:
    command = f'"${{PLUGIN_ROOT}}/hooks/{launcher_stem}'
    return {"hooks": [_hook(f'{command}.sh" {event}', f'{command}.cmd" {event}', 3)]}


def _bash_hooks(event: str) -> list[dict[str, object]]:
    return [_command_hook("^Bash$", "codexy-destructive-command", event)]


def _title_hooks(event: str) -> list[dict[str, object]]:
    return [_title_hook(matcher, kind, event) for matcher, kind in _TITLE_MATCHERS]


def _title_hook(matcher: str, kind: str, event: str) -> dict[str, object]:
    return {
        "matcher": matcher,
        "hooks": [
            _hook(
                f'"${{PLUGIN_ROOT}}/hooks/codexy-title-check.sh" {event} {kind}',
                f'"${{PLUGIN_ROOT}}/hooks/codexy-title-check.cmd" {event} {kind}',
                5,
            )
        ],
    }


_TITLE_MATCHERS = (
    (
        "^(?:mcp__codex_apps__github_(?:create|update)_issue|github\\.(?:create|update)_issue)$",
        "issue",
    ),
    (
        "^(?:mcp__codex_apps__github_(?:create|update)_pull_request|github\\.(?:create|update)_pull_request)$",
        "pr",
    ),
    (
        "^(?:mcp__codex_apps__github_(?:merge_pull_request|enable_auto_merge)|github\\.(?:merge_pull_request|enable_auto_merge))$",
        "merge",
    ),
    ("^functions\\.exec$", "nested"),
    ("^Bash$", "shell"),
)


def _core_hooks() -> dict[str, object]:
    command = lambda event: [
        _command_hook(matcher, stem, event) for matcher, stem in _CORE_COMMAND_HOOKS
    ]
    lifecycle = lambda event: [
        _command_hook(matcher, stem, event) for matcher, stem in _CORE_LIFECYCLE_HOOKS
    ]
    return {
        "hooks": {
            "PermissionRequest": command("PermissionRequest"),
            "PreToolUse": command("PreToolUse") + lifecycle("PreToolUse"),
            "Interrupt": [
                _lifecycle_hook(stem, "Interrupt") for _, stem in _CORE_LIFECYCLE_HOOKS
            ],
        }
    }


def _github_hooks() -> dict[str, object]:
    context = '"${PLUGIN_ROOT}/hooks/codexy-github-workflow-context'
    return {
        "hooks": {
            "UserPromptSubmit": [
                {"hooks": [_hook(f'{context}.sh"', f'{context}.cmd"', 5)]}
            ],
            "PermissionRequest": _title_hooks("PermissionRequest")
            + _bash_hooks("PermissionRequest"),
            "PreToolUse": _title_hooks("PreToolUse") + _bash_hooks("PreToolUse"),
        }
    }


HOOKS = {"core": _core_hooks(), "github": _github_hooks()}
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
                for name in _catalog_agent_files(
                    _text(plugin / "agents/catalog.toml", plugin)
                )
            )
            and all(_launcher(plugin / path, plugin) for path in LAUNCHERS[component])
            and _skill(plugin, component)
        )
    except (KeyError, OSError, UnicodeDecodeError, SyntaxError, ValueError):
        return False


def registration_role(
    name: str, state: str, cause: str | None = None, conflict: bool = False
) -> dict[str, object]:
    recovery = (
        "move or remove the unmanaged role file, then rerun getcodexy doctor"
        if conflict
        else REGISTRATION_REPAIR
        if cause
        else None
    )
    return {"file": name, "state": state, "cause": cause, "recovery": recovery}


def registration_report(
    component: str,
    observed: bool,
    roles: list[dict[str, object]],
    managed: tuple[str, ...],
    unmanaged: tuple[str, ...],
    expected_count: int,
    state: str,
    error: str | None = None,
) -> dict[str, object]:
    problem = next((item for item in roles if item["state"] != "exact"), None)
    cause = problem["cause"] if problem else error
    recovery = (
        problem["recovery"] if problem else REGISTRATION_REPAIR if cause else None
    )
    if not problem and unmanaged:
        cause = f"unmanaged role file is present: {unmanaged[0]}"
        recovery = "move or remove the unmanaged role file, then rerun getcodexy doctor"
    return {
        "component": component,
        "source": REGISTRATION_SOURCE,
        "scope": REGISTRATION_SCOPE,
        "observed": observed,
        "state": state,
        "roles": roles,
        "managed_extra": list(managed),
        "unmanaged_extra": list(unmanaged),
        "expected_count": expected_count,
        "exact_count": sum(item["state"] == "exact" for item in roles),
        "cause": cause,
        "recovery": recovery,
    }
