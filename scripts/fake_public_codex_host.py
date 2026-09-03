#!/usr/bin/env python3
"""Minimal Codex plugin host used only by the public release smoke proof."""

import json
import os
import sys
from pathlib import Path

root = Path(os.environ["CODEXY_MARKETPLACE_ROOT"])
home = Path(os.environ["CODEX_HOME"])
state_path = home / ".codexy-public-proof.json"
marketplace_path = home / ".codexy-public-marketplace-present"
state = (
    json.loads(state_path.read_text(encoding="utf-8"))
    if state_path.is_file()
    else {"selection": []}
)
target = os.environ["TARGET_VERSION"]
plugins = {"codexy": "core", "codexy-github": "github", "codexy-devtools": "devtools"}
command = sys.argv[1:]


def installed(name: str) -> dict[str, object]:
    plugin = root / "plugins" / name
    version = json.loads(
        (plugin / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
    )["version"]
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
    home.mkdir(parents=True, exist_ok=True)
    state_path.write_text(json.dumps(state), encoding="utf-8")
    result = {"ok": True}
elif command[:2] == ["plugin", "remove"]:
    name = command[2].split("@", 1)[0]
    state["selection"] = [item for item in state["selection"] if item != plugins[name]]
    state_path.write_text(json.dumps(state), encoding="utf-8")
    result = {"ok": True}
else:
    raise SystemExit(f"unexpected Codex command: {command!r}")
print(json.dumps(result))
