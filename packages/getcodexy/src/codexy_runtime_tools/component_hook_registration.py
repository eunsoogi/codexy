"""Canonical hook manifests for installed Codexy components."""

from .component_core_hooks import (
    COMMAND_HOOKS as _CORE_COMMAND_HOOKS,
    LIFECYCLE_HOOKS as _CORE_LIFECYCLE_HOOKS,
    LAUNCHERS as CORE_HOOK_LAUNCHERS,
)


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
            "UserPromptSubmit": [
                _lifecycle_hook(stem, "UserPromptSubmit")
                for _, stem in _CORE_LIFECYCLE_HOOKS
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
