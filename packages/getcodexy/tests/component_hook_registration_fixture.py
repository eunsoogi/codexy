"""Host hook rows built from explicit registration files for activation tests."""

import json
import os
from contextlib import contextmanager
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.updater import SyncResult


def hook_rows(plugin: Path) -> list[dict[str, object]]:
    events = {
        "PreToolUse": "preToolUse",
        "PermissionRequest": "permissionRequest",
        "UserPromptSubmit": "userPromptSubmit",
        "Interrupt": "interrupt",
    }
    event_keys = {
        "PreToolUse": "pre_tool_use",
        "PermissionRequest": "permission_request",
        "UserPromptSubmit": "user_prompt_submit",
        "Interrupt": "interrupt",
    }
    value = json.loads((plugin / "hooks/hooks.json").read_text(encoding="utf-8"))
    path = plugin / "hooks/hooks.json"
    rows = []
    for event, groups in value["hooks"].items():
        for group_index, group in enumerate(groups):
            for hook_index, hook in enumerate(group["hooks"]):
                command_key = "commandWindows" if os.name == "nt" else "command"
                command = hook[command_key].replace("${PLUGIN_ROOT}", str(plugin))
                rows.append(
                    {
                        "key": f"codexy@codexy:hooks/hooks.json:{event_keys[event]}:{group_index}:{hook_index}",
                        "eventName": events[event],
                        "handlerType": "command",
                        "command": command,
                        "async": hook.get("async", False),
                        "matcher": group.get("matcher"),
                        "timeoutSec": hook.get("timeout", 600),
                        "sourcePath": str(path),
                        "pluginId": "codexy@codexy",
                        "enabled": True,
                        "isManaged": False,
                        "currentHash": "sha256:fixture",
                        "trustStatus": "trusted",
                    }
                )
    return rows


def fixture_hook_rows(marketplace: Path) -> tuple[dict[str, object], ...]:
    rows = []
    for plugin in ("codexy", "codexy-github"):
        for row in hook_rows(marketplace / "plugins" / plugin):
            observed = row.copy()
            observed["key"] = observed["key"].replace(
                "codexy@codexy:", f"{plugin}@codexy:", 1
            )
            observed["pluginId"] = f"{plugin}@codexy"
            rows.append(observed)
    return tuple(rows)


def completed_registration(plugin, home, mode) -> SyncResult:
    return SyncResult(
        mode, "completed", "codexy", str(plugin), str(home), False, False, ()
    )


@contextmanager
def registration_context(real_registration: bool):
    if real_registration:
        yield
        return
    with patch(
        "codexy_runtime_tools.component_registration_health.sync_agents",
        side_effect=completed_registration,
    ):
        yield
