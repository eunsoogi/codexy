from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation


class LifecycleInterruptionTests(unittest.TestCase):
    def test_sigkill_mid_operation_recovers_from_journal_on_next_public_invocation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            home, market, state, codex = root / "home", root / "market", root / "state.json", root / "codex"
            market.mkdir()
            state.write_text(json.dumps(["core"]), encoding="utf-8")
            inventory = inventory_path(home)
            inventory.parent.mkdir(parents=True)
            inventory.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core"]}), encoding="utf-8")
            codex.write_text(_host_script(), encoding="utf-8")
            codex.chmod(0o700)
            environment = {**os.environ, "PYTHONPATH": str(Path(__file__).parents[1] / "src"), "KILL_PARENT": "1"}
            child = subprocess.run([sys.executable, "-c", _child_program(str(home), str(codex))], env=environment, check=False)

            self.assertNotEqual(child.returncode, 0)
            self.assertTrue((inventory.parent / "inflight.json").is_file())
            os.environ.pop("KILL_PARENT", None)
            try:
                receipt = run_operation("install", ("devtools",), home, codex, operation_id="op-after-kill")
            finally:
                os.environ.pop("KILL_PARENT", None)
            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(json.loads(state.read_text(encoding="utf-8")), ["core", "github", "devtools"])
            recovered = inventory.parent / "receipts" / "op-killed.json"
            self.assertEqual(json.loads(recovered.read_text(encoding="utf-8"))["outcome"], "completed")
            self.assertFalse((inventory.parent / "inflight.json").exists())


def _child_program(home: str, codex: str) -> str:
    return f"from codexy_runtime_tools.component_lifecycle import run_operation; run_operation('install', ('github',), {home!r}, __import__('pathlib').Path({codex!r}), operation_id='op-killed')"


def _host_script() -> str:
    return '''#!/usr/bin/env python3
import json, os, signal, sys
root = os.path.dirname(__file__)
state = os.path.join(root, "state.json")
market = os.path.join(root, "market")
names = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
reverse = {value: key for key, value in names.items()}
selected = json.load(open(state))
command = sys.argv[1:]
if command[:3] == ["plugin", "marketplace", "list"]:
    payload = {"marketplaces": [{"name": "codexy", "root": market, "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}]}
elif command[:2] == ["plugin", "list"]:
    payload = {"installed": [{"pluginId": names[item] + "@codexy", "name": names[item], "marketplaceName": "codexy", "version": "1.3.0", "installed": True, "enabled": True, "source": {"source": "local", "path": os.path.join(market, "plugins", names[item])}, "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}} for item in ["core", "github", "devtools"] if item in selected]}
else:
    item = reverse[command[2].split("@", 1)[0]]
    if command[1] == "add" and item not in selected: selected.append(item)
    if command[1] == "remove" and item in selected: selected.remove(item)
    json.dump(selected, open(state, "w"))
    if item == "github" and os.environ.get("KILL_PARENT"):
        os.kill(os.getppid(), signal.SIGKILL)
    payload = {"ok": True}
print(json.dumps(payload))
'''


if __name__ == "__main__":
    unittest.main()
