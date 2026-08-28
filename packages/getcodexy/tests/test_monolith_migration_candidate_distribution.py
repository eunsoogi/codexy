"""Installed-candidate migration proof against a separate locked target release."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.monolith_baseline import BASELINES, tree_digest
from codexy_runtime_tools.monolith_migration import migrate
from codexy_runtime_tools.monolith_migration_state import journal_path


TARGET_ROOT = os.environ.get("GETCODEXY_MIGRATION_CANDIDATE_ROOT")
LEGACY_ROOT = os.environ.get("GETCODEXY_MIGRATION_LEGACY_ROOT")


class CandidateMigrationDistributionTests(unittest.TestCase):
    def test_runtime_free_rejection_and_failed_activation_rollback(self) -> None:
        if not TARGET_ROOT or not LEGACY_ROOT:
            if os.environ.get("GETCODEXY_REQUIRE_MIGRATION_CANDIDATE") == "1":
                self.fail("required isolated migration candidate inputs are absent")
            self.skipTest("isolated migration candidate is required")
        candidate = Path(TARGET_ROOT or "").resolve()
        legacy = Path(LEGACY_ROOT or "").resolve()
        self.assertEqual(tree_digest(legacy), BASELINES["1.3.0"].tree_sha256)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / "active-home"
            home.mkdir()
            config = b'[marketplaces.codexy]\nref = "v1.3.0"\nmode = "preserve"\n'
            (home / "config.toml").write_bytes(config)
            agents = home / "agents/codexy"
            agents.mkdir(parents=True)
            (agents / "custom.toml").write_text("keep = true\n", encoding="utf-8")
            os.chmod(agents, 0o700)
            os.chmod(agents / "custom.toml", 0o600)
            state = root / "host-state.json"
            state.write_text(
                json.dumps(
                    {
                        "active_home": str(home),
                        "candidate": str(candidate),
                        "legacy": str(legacy.parent.parent),
                        "fail_active_component": "github",
                        "homes": {str(home): {"ref": "v1.3.0", "selection": ["core"]}},
                    }
                ),
                encoding="utf-8",
            )
            codex = _write_host(root, state)
            environment = os.environ.copy()
            environment["GETCODEXY_CANDIDATE_HOST_STATE"] = str(state)
            failure = _migrate(home, codex, environment)
            self.assertEqual(failure["outcome"], "rejected")
            self.assertEqual(failure["source_version"], "1.3.0")
            self.assertEqual(failure["target_version"], "1.4.0")
            self.assertEqual(failure["selection_after"], [])
            self.assertEqual(
                failure["errors"], [{"code": "target-release-unavailable"}]
            )
            self.assertEqual((home / "config.toml").read_bytes(), config)
            self.assertEqual(
                (agents / "custom.toml").read_text(encoding="utf-8"), "keep = true\n"
            )
            self.assertEqual((agents / "custom.toml").stat().st_mode & 0o777, 0o600)
            self.assertFalse(journal_path(home).exists())
            recovered = json.loads(state.read_text(encoding="utf-8"))
            self.assertEqual(
                recovered["homes"][str(home)], {"ref": "v1.3.0", "selection": ["core"]}
            )

            complete_selection = ("core", "github")
            rollback = _migrate(home, codex, environment, requested=complete_selection)
            self.assertEqual(rollback["outcome"], "rolled-back")
            self.assertEqual(rollback["source_version"], "1.3.0")
            self.assertEqual(rollback["target_version"], failure["target_version"])
            self.assertEqual(rollback["selection_after"], [])
            self.assertEqual(rollback["errors"], [{"code": "operation-failed"}])
            self.assertEqual((home / "config.toml").read_bytes(), config)
            self.assertEqual(
                (agents / "custom.toml").read_text(encoding="utf-8"), "keep = true\n"
            )
            self.assertEqual((agents / "custom.toml").stat().st_mode & 0o777, 0o600)
            self.assertFalse(journal_path(home).exists())
            recovered = json.loads(state.read_text(encoding="utf-8"))
            self.assertEqual(
                recovered["homes"][str(home)], {"ref": "v1.3.0", "selection": ["core"]}
            )

            success = _migrate(home, codex, environment, requested=complete_selection)
            self.assertEqual(success["outcome"], "completed")
            self.assertEqual(success["source_version"], "1.3.0")
            self.assertEqual(success["target_version"], "1.4.0")
            self.assertEqual(success["selection_after"], ["core", "github"])
            completed = json.loads(state.read_text(encoding="utf-8"))
            self.assertEqual(
                completed["homes"][str(home)],
                {"ref": "v1.4.0", "selection": ["core", "github"]},
            )
            self.assertFalse(journal_path(home).exists())


def _migrate(
    home: Path,
    codex: Path,
    environment: dict[str, str],
    requested: tuple[str, ...] = (),
) -> dict[str, object]:
    previous = os.environ.get("GETCODEXY_CANDIDATE_HOST_STATE")
    try:
        os.environ.update(environment)
        receipt = migrate(
            home, codex, lambda command: _run(command, home), requested=requested
        )
    finally:
        if previous is None:
            os.environ.pop("GETCODEXY_CANDIDATE_HOST_STATE", None)
        else:
            os.environ["GETCODEXY_CANDIDATE_HOST_STATE"] = previous
    return receipt


def _run(command: list[str], home: Path) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["CODEX_HOME"] = str(home)
    return subprocess.run(
        command, text=True, capture_output=True, check=False, env=environment
    )


def _write_host(root: Path, state: Path) -> Path:
    executable = root / "codex"
    executable.write_text(_HOST, encoding="utf-8")
    executable.chmod(0o700)
    return executable


_HOST = r"""#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

state_path = Path(os.environ["GETCODEXY_CANDIDATE_HOST_STATE"])
state = json.loads(state_path.read_text(encoding="utf-8"))
home = os.environ["CODEX_HOME"]
entry = state["homes"].setdefault(home, {"ref": None, "selection": []})
command = sys.argv[1:]
plugins = {"codexy": "core", "codexy-github": "github", "codexy-devtools": "devtools"}

def save():
    state_path.write_text(json.dumps(state), encoding="utf-8")

def marketplace():
    if entry["ref"] is None:
        return {"marketplaces": []}
    root = state["legacy"] if entry["ref"] == "v1.3.0" else state["candidate"]
    return {"marketplaces": [{"name": "codexy", "root": root,
        "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}]}

def inventory():
    root = state["legacy"] if entry["ref"] == "v1.3.0" else state["candidate"]
    version = "1.3.0" if entry["ref"] == "v1.3.0" else "1.4.0"
    items = []
    for component in entry["selection"]:
        plugin = next(name for name, value in plugins.items() if value == component)
        items.append({"pluginId": f"{plugin}@codexy", "name": plugin, "marketplaceName": "codexy",
            "version": version, "installed": True, "enabled": True,
            "source": {"source": "local", "path": str(Path(root) / "plugins" / plugin)},
            "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}})
    return {"installed": items}

if command == ["plugin", "marketplace", "list", "--json"]:
    payload = marketplace()
elif command[:3] == ["plugin", "marketplace", "add"]:
    entry["ref"] = command[command.index("--ref") + 1]
    Path(home).mkdir(parents=True, exist_ok=True)
    (Path(home) / "config.toml").write_text(f'[marketplaces.codexy]\nref = "{entry["ref"]}"\n', encoding="utf-8")
    save()
    payload = {"ok": True}
elif command == ["plugin", "marketplace", "remove", "codexy", "--json"]:
    entry["ref"] = None
    save()
    payload = {"ok": True}
elif command == ["plugin", "marketplace", "upgrade", "codexy", "--json"]:
    payload = {"ok": True}
elif command == ["plugin", "list", "--json"]:
    payload = inventory()
elif command[:2] == ["plugin", "add"]:
    component = plugins[command[2].split("@", 1)[0]]
    if home == state["active_home"] and component == state.get("fail_active_component"):
        state["fail_active_component"] = None
        save()
        print(json.dumps({"error": "injected"}))
        raise SystemExit(1)
    if component not in entry["selection"]:
        entry["selection"].append(component)
    save()
    payload = {"ok": True}
elif command[:2] == ["plugin", "remove"]:
    component = plugins[command[2].split("@", 1)[0]]
    entry["selection"] = [item for item in entry["selection"] if item != component]
    save()
    payload = {"ok": True}
else:
    payload = {"ok": True}
print(json.dumps(payload))
"""


if __name__ == "__main__":
    unittest.main()
