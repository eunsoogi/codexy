"""Build the exact hook registrations required by installed components."""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from .component_hook_activation_host import HookStateError
from .component_manifest import ComponentManifest


_EVENT_LABELS = {
    "PreToolUse": "preToolUse",
    "PermissionRequest": "permissionRequest",
    "UserPromptSubmit": "userPromptSubmit",
    "SessionStart": "sessionStart",
    "Stop": "stop",
}
_EVENT_KEYS = {
    "PreToolUse": "pre_tool_use",
    "PermissionRequest": "permission_request",
    "UserPromptSubmit": "user_prompt_submit",
    "SessionStart": "session_start",
    "Stop": "stop",
}


@dataclass(frozen=True)
class ExpectedHook:
    component: str
    plugin_id: str
    version: str
    repository: str
    source_path: Path
    event_key: str
    event_name: str
    group_index: int
    hook_index: int
    matcher: object
    command: str
    timeout: int
    asynchronous: bool

    @property
    def key(self) -> str:
        return (
            f"{self.plugin_id}:hooks/hooks.json:{self.event_key}:"
            f"{self.group_index}:{self.hook_index}"
        )


def expected_hooks(
    manifest: ComponentManifest,
    components: tuple[str, ...],
    records: dict[str, dict[str, object]],
) -> dict[str, tuple[ExpectedHook, ...]]:
    expected: dict[str, tuple[ExpectedHook, ...]] = {}
    for component in components:
        record = records.get(component)
        source = record.get("source") if isinstance(record, dict) else None
        source_map = cast(dict[str, object], source) if isinstance(source, dict) else {}
        path_value = source_map.get("path")
        if not isinstance(path_value, str) or not Path(path_value).is_absolute():
            raise HookStateError("installed hook source path is not absolute")
        plugin = Path(path_value)
        hooks_path = plugin / "hooks/hooks.json"
        try:
            value = cast(object, json.loads(hooks_path.read_text(encoding="utf-8")))
        except (OSError, UnicodeError, ValueError) as error:
            raise HookStateError("installed hook registration is unreadable") from error
        if not isinstance(value, dict):
            raise HookStateError("installed hook registration has an invalid shape")
        value_map = cast(dict[str, object], value)
        hooks = value_map.get("hooks")
        if not isinstance(hooks, dict):
            raise HookStateError("installed hook registration has an invalid shape")
        hooks_map = cast(dict[str, object], hooks)
        entries: list[ExpectedHook] = []
        for event, groups_value in hooks_map.items():
            if event not in _EVENT_LABELS or event not in _EVENT_KEYS:
                raise HookStateError("installed hook registration has an unknown event")
            if not isinstance(groups_value, list):
                raise HookStateError("installed hook registration has invalid groups")
            groups = cast(list[object], groups_value)
            for group_index, group_value in enumerate(groups):
                if not isinstance(group_value, dict):
                    raise HookStateError(
                        "installed hook registration has invalid group"
                    )
                group = cast(dict[str, object], group_value)
                group_hooks = group.get("hooks")
                if not isinstance(group_hooks, list):
                    raise HookStateError(
                        "installed hook registration has invalid group"
                    )
                matcher = group.get("matcher")
                for hook_index, hook_value in enumerate(
                    cast(list[object], group_hooks)
                ):
                    if not isinstance(hook_value, dict):
                        raise HookStateError(
                            "installed hook registration contains a non-command hook"
                        )
                    hook = cast(dict[str, object], hook_value)
                    if hook.get("type") != "command":
                        raise HookStateError(
                            "installed hook registration contains a non-command hook"
                        )
                    command_key = "commandWindows" if os.name == "nt" else "command"
                    command = hook.get(command_key)
                    if not isinstance(command, str):
                        raise HookStateError("installed hook command is missing")
                    timeout = hook.get("timeout", 600)
                    if not isinstance(timeout, int) or isinstance(timeout, bool):
                        raise HookStateError("installed hook timeout is invalid")
                    asynchronous = hook.get("async", False)
                    if not isinstance(asynchronous, bool):
                        raise HookStateError("installed hook async flag is invalid")
                    entries.append(
                        ExpectedHook(
                            component,
                            manifest.component(component).asset.plugin_id,
                            manifest.version,
                            manifest.marketplace.source,
                            hooks_path,
                            _EVENT_KEYS[event],
                            _EVENT_LABELS[event],
                            group_index,
                            hook_index,
                            matcher,
                            command.replace("${PLUGIN_ROOT}", str(plugin)),
                            timeout,
                            asynchronous,
                        )
                    )
        expected[component] = tuple(entries)
    return expected
