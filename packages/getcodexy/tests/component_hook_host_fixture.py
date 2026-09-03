"""Fake app-server host for effective hook registry tests."""

import os
import sys
from pathlib import Path


def write_host(root: Path) -> Path:
    host = root / "trusted/codex-host.py"
    codex = root / ("trusted/codex.cmd" if os.name == "nt" else "trusted/codex")
    codex.parent.mkdir(parents=True)
    host.write_text(HOOK_LIST_HOST, encoding="utf-8")
    if os.name == "nt":
        codex.write_text(
            f'@echo off\r\n"{sys.executable}" "{host}" %*\r\n',
            encoding="utf-8",
        )
    else:
        codex.write_text(
            f'#!/bin/sh\nexec "{sys.executable}" "{host}" "$@"\n',
            encoding="utf-8",
        )
        codex.chmod(0o700)
    return codex


HOOK_LIST_HOST = r"""#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

executable = Path(sys.argv[0]).resolve()
root = next(
    (
        candidate / directory
        for candidate in (executable.parent, executable.parent.parent)
        for directory in ("marketplace", "candidate-marketplace")
        if (candidate / directory).exists()
    ),
    executable.parent / "marketplace",
)
plugins = {
    "core": "codexy",
    "github": "codexy-github",
}
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

def hook_rows():
    rows = []
    for plugin_id, plugin_name in plugins.items():
        plugin = root / "plugins" / plugin_name
        path = plugin / "hooks" / "hooks.json"
        if not path.is_file():
            continue
        value = json.loads(path.read_text())
        for event, groups in value["hooks"].items():
            for group_index, group in enumerate(groups):
                for hook_index, hook in enumerate(group["hooks"]):
                    command_key = "commandWindows" if os.name == "nt" else "command"
                    command = hook[command_key].replace("${PLUGIN_ROOT}", str(plugin))
                    rows.append({
                        "key": f"{plugin_name}@codexy:hooks/hooks.json:{event_keys[event]}:{group_index}:{hook_index}",
                        "eventName": events[event],
                        "handlerType": "command",
                        "command": command,
                        "async": hook.get("async", False),
                        "matcher": group.get("matcher"),
                        "timeoutSec": hook.get("timeout", 600),
                        "sourcePath": str(path),
                        "pluginId": f"{plugin_name}@codexy",
                        "enabled": True,
                        "isManaged": False,
                        "currentHash": "sha256:fixture",
                        "trustStatus": "trusted",
                    })
    return rows

if sys.argv[1:4] == ["app-server", "--listen", "stdio://"]:
    for line in sys.stdin:
        request = json.loads(line)
        identifier = request.get("id")
        if request.get("method") == "initialize" and identifier is not None:
            result = {"userAgent": "fixture", "codexHome": "fixture"}
        elif request.get("method") == "hooks/list" and identifier is not None:
            cwds = request.get("params", {}).get("cwds", [])
            result = {"data": [{"cwd": cwds[0] if cwds else ".", "hooks": hook_rows(), "warnings": [], "errors": []}]}
        else:
            continue
        print(json.dumps({"jsonrpc": "2.0", "id": identifier, "result": result}), flush=True)
    raise SystemExit(0)
raise SystemExit(0)
"""
