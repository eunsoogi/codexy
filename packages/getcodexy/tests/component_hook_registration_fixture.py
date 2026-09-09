"""Host hook rows built from explicit registration files for activation tests."""

import json
import os
from pathlib import Path


def hook_rows(plugin: Path) -> list[dict[str, object]]:
    events = {
        "PreToolUse": "preToolUse",
        "PermissionRequest": "permissionRequest",
        "UserPromptSubmit": "userPromptSubmit",
    }
    event_keys = {
        "PreToolUse": "pre_tool_use",
        "PermissionRequest": "permission_request",
        "UserPromptSubmit": "user_prompt_submit",
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
