from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest


class LifecycleInterruptionTests(unittest.TestCase):
    @unittest.skipIf(os.name == "nt", "SIGKILL interruption is POSIX-specific")
    def test_sigkill_mid_operation_recovers_from_journal_on_next_public_invocation(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            home, market, state, codex = (
                root / "home",
                root / "market",
                root / "state.json",
                root / "codex",
            )
            market.mkdir()
            repository = Path(__file__).parents[3]
            for plugin in ("codexy", "codexy-github", "codexy-devtools"):
                shutil.copytree(
                    repository / "plugins" / plugin,
                    market / "plugins" / plugin,
                )
            state.write_text(json.dumps(["core"]), encoding="utf-8")
            inventory = inventory_path(home)
            inventory.parent.mkdir(parents=True)
            inventory.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core"],
                    }
                ),
                encoding="utf-8",
            )
            codex.write_text(
                _host_script(load_component_manifest().version), encoding="utf-8"
            )
            codex.chmod(0o700)
            environment = {
                **os.environ,
                "PYTHONPATH": str(Path(__file__).parents[1] / "src"),
                "KILL_PARENT": "1",
            }
            child = subprocess.run(
                [sys.executable, "-c", _child_program(str(home), str(codex))],
                env=environment,
                check=False,
            )

            self.assertNotEqual(child.returncode, 0)
            self.assertTrue((inventory.parent / "inflight.json").is_file())
            os.environ.pop("KILL_PARENT", None)
            try:
                receipt = run_operation(
                    "install", ("devtools",), home, codex, operation_id="op-after-kill"
                )
            finally:
                os.environ.pop("KILL_PARENT", None)
            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(
                json.loads(state.read_text(encoding="utf-8")),
                ["core", "github", "devtools"],
            )
            recovered = inventory.parent / "receipts" / "op-killed.json"
            self.assertEqual(
                json.loads(recovered.read_text(encoding="utf-8"))["outcome"],
                "completed",
            )
            self.assertFalse((inventory.parent / "inflight.json").exists())


def _child_program(home: str, codex: str) -> str:
    return f"from codexy_runtime_tools.component_lifecycle import run_operation; run_operation('install', ('github',), {home!r}, __import__('pathlib').Path({codex!r}), operation_id='op-killed')"


def _host_script(version: str) -> str:
    return """#!/usr/bin/env python3
import json, os, signal, sys
from pathlib import Path
root = os.path.dirname(__file__)
state = os.path.join(root, "state.json")
market = os.path.join(root, "market")
names = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
reverse = {value: key for key, value in names.items()}
selected = json.load(open(state))
command = sys.argv[1:]
events = {"PreToolUse": "preToolUse", "PermissionRequest": "permissionRequest", "UserPromptSubmit": "userPromptSubmit"}
event_keys = {"PreToolUse": "pre_tool_use", "PermissionRequest": "permission_request", "UserPromptSubmit": "user_prompt_submit"}

def hook_rows():
    rows = []
    for component in selected:
        plugin_name = names[component]
        plugin = Path(market) / "plugins" / plugin_name
        path = plugin / "hooks" / "hooks.json"
        if not path.is_file():
            continue
        value = json.loads(path.read_text())
        for event, groups in value["hooks"].items():
            for group_index, group in enumerate(groups):
                for hook_index, hook in enumerate(group["hooks"]):
                    rows.append({
                        "key": f"{plugin_name}@codexy:hooks/hooks.json:{event_keys[event]}:{group_index}:{hook_index}",
                        "eventName": events[event],
                        "handlerType": "command",
                        "command": hook["command"].replace("${PLUGIN_ROOT}", str(plugin)),
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

if command[:3] == ["app-server", "--listen", "stdio://"]:
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

if command[:3] == ["plugin", "marketplace", "list"]:
    payload = {"marketplaces": [{"name": "codexy", "root": market, "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}]}
elif command[:2] == ["plugin", "list"]:
    payload = {"installed": [{"pluginId": names[item] + "@codexy", "name": names[item], "marketplaceName": "codexy", "version": "__VERSION__", "installed": True, "enabled": True, "source": {"source": "local", "path": os.path.join(market, "plugins", names[item])}, "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}} for item in ["core", "github", "devtools"] if item in selected]}
else:
    item = reverse[command[2].split("@", 1)[0]]
    if command[1] == "add" and item not in selected: selected.append(item)
    if command[1] == "remove" and item in selected: selected.remove(item)
    json.dump(selected, open(state, "w"))
    if item == "github" and os.environ.get("KILL_PARENT"):
        os.kill(os.getppid(), signal.SIGKILL)
    payload = {"ok": True}
print(json.dumps(payload))
""".replace("__VERSION__", version)


if __name__ == "__main__":
    unittest.main()
