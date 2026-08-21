"""Installed-wheel lifecycle matrix against a stateful temporary Codex host."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.version_lock import default_package_version
from packages.getcodexy.tests.component_distribution_support import (
    copy_marketplace_plugins,
)


EXECUTABLE_ENV = "GETCODEXY_DISTRIBUTION_EXECUTABLE"
REPOSITORY = Path(__file__).parents[3]


class ComponentDistributionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        value = os.environ.get(EXECUTABLE_ENV)
        if not value:
            raise unittest.SkipTest(f"{EXECUTABLE_ENV} is not set")
        cls.executable = Path(value).resolve()
        if not cls.executable.is_file():
            raise RuntimeError(
                f"missing installed getcodexy executable: {cls.executable}"
            )

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.home = self.root / "home"
        self.marketplace = self.root / "marketplace"
        self.state = self.root / "host-state.json"
        self.host = self.root / "codex-host.py"
        self.codex = self.root / ("codex.cmd" if os.name == "nt" else "codex")
        self.version = copy_marketplace_plugins(REPOSITORY, self.marketplace)
        self.state.write_text(json.dumps({"marketplace": False, "selection": []}))
        self.host.write_text(_HOST, encoding="utf-8")
        if os.name == "nt":
            self.codex.write_text(
                f'@echo off\r\n"{sys.executable}" "{self.host}" %*\r\n',
                encoding="utf-8",
            )
        else:
            self.codex.write_text(
                f'#!/bin/sh\nexec "{sys.executable}" "{self.host}" "$@"\n',
                encoding="utf-8",
            )
            self.codex.chmod(self.codex.stat().st_mode | stat.S_IXUSR)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_installed_cli_executes_supported_lifecycle_matrix(self) -> None:
        self.assertEqual(
            self._run("install")["selection_after"], ["core", "github", "devtools"]
        )
        self.assertEqual(
            self._run("status")["installed_components"], ["core", "github", "devtools"]
        )
        doctor = self._run("doctor")
        self.assertEqual(doctor["inventory_consistency"], "consistent")
        self.assertEqual(
            _health(doctor),
            {"core": "healthy", "github": "healthy", "devtools": "healthy"},
        )
        self.assertEqual(
            self._run("update", "github")["selection_after"],
            ["core", "github", "devtools"],
        )
        rejected = self._run("remove", "core", expected=2)
        self.assertEqual(rejected["errors"], [{"code": "dependency-protected-removal"}])
        self.assertEqual(
            self._run("remove", "github")["selection_after"], ["core", "devtools"]
        )
        self.assertEqual(
            self._run("install", "github")["selection_after"],
            ["core", "github", "devtools"],
        )

    def test_packaged_manifest_drives_install_and_update_at_package_version(
        self,
    ) -> None:
        self.assertEqual(
            self._run("install")["selection_after"],
            ["core", "github", "devtools"],
        )
        self.assertEqual(
            self._run("update", "github")["selection_after"],
            ["core", "github", "devtools"],
        )
        self.assertEqual(load_component_manifest().version, default_package_version())

    def test_installed_cli_detects_an_incomplete_plugin_package(self) -> None:
        self._run("install")
        (self.marketplace / "plugins/codexy-devtools/.mcp.json").unlink()
        doctor = self._run("doctor")
        self.assertEqual(_health(doctor)["devtools"], "stale")

    def test_installed_cli_rejects_incomplete_core_and_github_surfaces(self) -> None:
        self._run("install", "core")
        self.assertEqual(_health(self._run("doctor")), {"core": "healthy"})
        (self.marketplace / "plugins/codexy/skills/wiki/SKILL.md").unlink()
        self.assertEqual(_health(self._run("doctor"))["core"], "stale")
        (self.marketplace / "plugins/codexy/skills/wiki/SKILL.md").write_text("x")
        self.assertEqual(_health(self._run("doctor"))["core"], "incompatible")

    def test_installed_cli_rejects_empty_specialist_and_hook(self) -> None:
        self._run("install", "github")
        (self.marketplace / "plugins/codexy/agents/codexy-sentinel.toml").write_text(
            "x"
        )
        self.assertEqual(_health(self._run("doctor"))["core"], "incompatible")
        (
            self.marketplace / "plugins/codexy-github/hooks/codexy-github-admission.sh"
        ).write_text("x")
        self.assertEqual(_health(self._run("doctor"))["github"], "incompatible")

    def test_installed_cli_bootstrap_and_invalid_migration_are_fail_closed(
        self,
    ) -> None:
        before = self.state.read_bytes()
        receipt = self._run("migrate", expected=2)
        self.assertEqual(receipt["errors"], [{"code": "ambiguous-monolith"}])
        self.assertEqual(self.state.read_bytes(), before)
        self.assertEqual(
            self._run("bootstrap")["selection_after"], ["core", "github", "devtools"]
        )
        self.assertEqual(
            _health(self._run("doctor")),
            {"core": "healthy", "github": "healthy", "devtools": "healthy"},
        )

    def test_installed_cli_rolls_back_a_failed_add(self) -> None:
        self._run("install", "github")
        state = json.loads(self.state.read_text(encoding="utf-8"))
        state["fail_add"] = "codexy-devtools"
        self.state.write_text(json.dumps(state), encoding="utf-8")
        receipt = self._run("install", "devtools", expected=2)
        self.assertEqual(receipt["outcome"], "rolled-back")
        self.assertEqual(receipt["selection_after"], ["core", "github"])
        self.assertEqual(
            self._run("status")["installed_components"], ["core", "github"]
        )

    def _run(
        self, command: str, *components: str, expected: int = 0
    ) -> dict[str, object]:
        environment = os.environ | {
            "CODEXY_MATRIX_STATE": str(self.state),
            "CODEXY_MATRIX_MARKETPLACE": str(self.marketplace),
            "CODEXY_MATRIX_VERSION": self.version,
        }
        result = subprocess.run(
            [
                self.executable,
                "--codex",
                self.codex,
                "--codex-home",
                self.home,
                command,
                *components,
                "--json",
            ],
            text=True,
            capture_output=True,
            check=False,
            env=environment,
        )
        self.assertEqual(result.returncode, expected, result.stderr + result.stdout)
        return json.loads(result.stdout)


def _health(receipt: dict[str, object]) -> dict[str, str]:
    entries = receipt["component_health"]
    assert isinstance(entries, list)
    return {
        entry["component"]: entry["state"]
        for entry in entries
        if isinstance(entry, dict)
        and isinstance(entry.get("component"), str)
        and isinstance(entry.get("state"), str)
    }


_HOST = """#!/usr/bin/env python3
import json, os, sys
from pathlib import Path

state_path = Path(os.environ["CODEXY_MATRIX_STATE"])
root = Path(os.environ["CODEXY_MATRIX_MARKETPLACE"]).resolve()
version = os.environ["CODEXY_MATRIX_VERSION"]
state = json.loads(state_path.read_text())
plugins = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
reverse = {value: key for key, value in plugins.items()}
args = sys.argv[1:]

def save(): state_path.write_text(json.dumps(state))
def installed(component):
    plugin = plugins[component]
    return {"pluginId": plugin + "@codexy", "name": plugin, "marketplaceName": "codexy", "version": version, "installed": True, "enabled": True, "source": {"source": "local", "path": str(root / "plugins" / plugin)}, "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}

if args[:4] == ["plugin", "marketplace", "list", "--json"]:
    payload = {"marketplaces": [] if not state["marketplace"] else [{"name": "codexy", "root": str(root), "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}]}
elif args[:3] == ["plugin", "marketplace", "add"]:
    state["marketplace"] = True; save(); payload = {"ok": True}
elif args[:3] == ["plugin", "marketplace", "upgrade"]:
    payload = {"ok": True}
elif args[:3] == ["plugin", "marketplace", "remove"]:
    state["marketplace"] = False; save(); payload = {"ok": True}
elif args[:3] == ["plugin", "list", "--json"]:
    payload = {"installed": [installed(component) for component in ("core", "github", "devtools") if component in state["selection"]]}
elif args[:2] == ["plugin", "add"]:
    plugin = args[2].split("@", 1)[0]
    if state.get("fail_add") == plugin:
        state.pop("fail_add"); save(); print(json.dumps({"error": "injected"})); raise SystemExit(1)
    if reverse[plugin] not in state["selection"]: state["selection"].append(reverse[plugin])
    save(); payload = {"ok": True}
elif args[:2] == ["plugin", "remove"]:
    component = reverse[args[2].split("@", 1)[0]]
    if component in state["selection"]: state["selection"].remove(component)
    save(); payload = {"ok": True}
else:
    payload = {"ok": True}
print(json.dumps(payload))
"""


if __name__ == "__main__":
    unittest.main()
