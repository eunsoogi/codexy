#!/usr/bin/env python3
"""Minimal Codex plugin host used only by the public release smoke proof."""

import hashlib
import json
import os
import sys
from pathlib import Path

root = Path(os.environ["CODEXY_MARKETPLACE_ROOT"])
home = Path(os.environ["CODEX_HOME"])
state_path = home / ".codexy-public-proof.json"
marketplace_path = home / ".codexy-public-marketplace-present"
target = os.environ["TARGET_VERSION"]
plugins = {"codexy": "core", "codexy-github": "github", "codexy-devtools": "devtools"}
state = (
    json.loads(state_path.read_text(encoding="utf-8"))
    if state_path.is_file()
    else {"selection": [], "versions": {}}
)
state.setdefault("versions", {})
command = sys.argv[1:]


EVENTS = {
    "PreToolUse": ("preToolUse", "pre_tool_use"),
    "PermissionRequest": ("permissionRequest", "permission_request"),
    "UserPromptSubmit": ("userPromptSubmit", "user_prompt_submit"),
}


def hook_rows() -> list[dict[str, object]]:
    rows = []
    for name, component in plugins.items():
        if component not in state["selection"] or component == "devtools":
            continue
        plugin = (root / "plugins" / name).resolve()
        path = plugin / "hooks/hooks.json"
        content = path.read_bytes()
        definitions = json.loads(content)["hooks"]
        for event, groups in definitions.items():
            event_name, event_key = EVENTS[event]
            for group_index, group in enumerate(groups):
                for hook_index, hook in enumerate(group["hooks"]):
                    if hook["type"] != "command":
                        raise ValueError("unsupported hook handler")
                    command_key = "commandWindows" if os.name == "nt" else "command"
                    rows.append(
                        {
                            "key": f"{name}@codexy:hooks/hooks.json:{event_key}:{group_index}:{hook_index}",
                            "eventName": event_name,
                            "handlerType": "command",
                            "command": hook[command_key].replace(
                                "${PLUGIN_ROOT}", str(plugin)
                            ),
                            "async": hook.get("async", False),
                            "matcher": group.get("matcher"),
                            "timeoutSec": hook.get("timeout", 600),
                            "sourcePath": str(path),
                            "pluginId": f"{name}@codexy",
                            "enabled": True,
                            "isManaged": False,
                            "currentHash": "sha256:"
                            + hashlib.sha256(content).hexdigest(),
                            "trustStatus": "trusted",
                        }
                    )
    return rows


def app_server() -> None:
    for line in sys.stdin:
        request = json.loads(line)
        identifier = request.get("id")
        if identifier is None:
            continue
        response = {"jsonrpc": "2.0", "id": identifier}
        try:
            if request.get("method") == "initialize":
                result = {"userAgent": "public-smoke-host", "codexHome": str(home)}
            elif request.get("method") == "hooks/list":
                result = {
                    "data": [
                        {"cwd": cwd, "hooks": hook_rows(), "warnings": [], "errors": []}
                        for cwd in request["params"]["cwds"]
                    ]
                }
            else:
                raise ValueError("unsupported app-server method")
            response["result"] = result
        except (OSError, ValueError, KeyError, TypeError) as error:
            response["error"] = {"code": -32603, "message": str(error)}
        print(json.dumps(response), flush=True)


if command == ["app-server", "--listen", "stdio://"]:
    app_server()
    raise SystemExit(0)


def installed(name: str) -> dict[str, object]:
    plugin = root / "plugins" / name
    packaged_version = json.loads(
        (plugin / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
    )["version"]
    version = state["versions"].get(plugins[name], packaged_version)
    return {
        "pluginId": f"{name}@codexy",
        "name": name,
        "marketplaceName": "codexy",
        "version": version,
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": str(plugin.resolve())},
        "marketplaceSource": {
            "sourceType": "git",
            "source": "https://github.com/eunsoogi/codexy.git",
        },
    }


if command == ["plugin", "marketplace", "list", "--json"]:
    marketplaces = (
        [
            {
                "name": "codexy",
                "root": str(root.resolve()),
                "marketplaceSource": {
                    "sourceType": "git",
                    "source": "https://github.com/eunsoogi/codexy.git",
                },
            }
        ]
        if marketplace_path.is_file()
        else []
    )
    result = {"marketplaces": marketplaces}
elif command[:3] == ["plugin", "marketplace", "add"]:
    home.mkdir(parents=True, exist_ok=True)
    (home / "config.toml").write_text(
        f'[marketplaces.codexy]\nref = "v{target}"\n', encoding="utf-8"
    )
    marketplace_path.write_text("present", encoding="utf-8")
    result = {"ok": True}
elif command == ["plugin", "marketplace", "remove", "codexy", "--json"]:
    marketplace_path.unlink(missing_ok=True)
    result = {"ok": True}
elif command == ["plugin", "marketplace", "upgrade", "codexy", "--json"]:
    if os.environ.get("FAIL_MARKETPLACE_UPGRADE") == "1":
        raise SystemExit("unexpected unpinned marketplace upgrade")
    result = {"ok": True}
elif command == ["plugin", "list", "--json"]:
    result = {
        "installed": [
            installed(name)
            for name, component in plugins.items()
            if component in state["selection"]
        ]
    }
elif command[:2] == ["plugin", "add"]:
    name = command[2].split("@", 1)[0]
    if name not in plugins:
        raise SystemExit(f"unknown plugin: {name}")
    if plugins[name] not in state["selection"]:
        state["selection"].append(plugins[name])
    state["versions"][plugins[name]] = target
    home.mkdir(parents=True, exist_ok=True)
    state_path.write_text(json.dumps(state), encoding="utf-8")
    result = {"ok": True}
elif command[:2] == ["plugin", "remove"]:
    name = command[2].split("@", 1)[0]
    state["selection"] = [item for item in state["selection"] if item != plugins[name]]
    state["versions"].pop(plugins[name], None)
    state_path.write_text(json.dumps(state), encoding="utf-8")
    result = {"ok": True}
else:
    raise SystemExit(f"unexpected Codex command: {command!r}")
print(json.dumps(result))
