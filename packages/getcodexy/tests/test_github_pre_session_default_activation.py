from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.component_integrity import frozen_component
from codexy_runtime_tools.github_pre_session import run_github_pre_session


REPOSITORY = Path(__file__).resolve().parents[3]
OFFICIAL = "https://github.com/eunsoogi/codexy.git"


class GithubPreSessionDefaultActivationTests(unittest.TestCase):
    def test_frozen_bundle_rejects_a_manifest_version_not_matching_host_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            component = copy_plugin(Path(temporary), "codexy-github")
            with self.assertRaisesRegex(ValueError, "manifest version mismatch"):
                with frozen_component(component, "codexy-github", "0.0.0"):
                    pass

    def test_real_default_activators_run_from_complete_verified_bundles(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marketplace = root / "marketplace"
            core = copy_plugin(marketplace, "codexy")
            github = copy_plugin(marketplace, "codexy-github")
            codex = executable(root)
            list_calls = 0

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                nonlocal list_calls
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    payload: object = {"marketplaces": [marketplace_entry(marketplace)]}
                elif command[1:3] == ["plugin", "list"]:
                    list_calls += 1
                    payload = {"installed": [] if list_calls == 1 else [installed(core, "codexy"), installed(github, "codexy-github")]}
                else:
                    payload = {"ok": True}
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            home = root / "fresh Codex home"
            result = run_github_pre_session(home, codex=codex, runner=runner, package_version="1.3.0")

            self.assertTrue(result.changed)
            self.assertTrue((home / "agents/codexy/codexy-sentinel.toml").is_file())
            self.assertTrue((home / "agents/codexy-github/codexy-weaver.toml").is_file())

    def test_tampered_host_manifest_content_fails_and_rolls_back(self) -> None:
        cases = (
            ("core MCP", "codexy", lambda data: data.__setitem__("mcpServers", "/tmp/untrusted-mcp.json")),
            ("GitHub skills", "codexy-github", lambda data: data.__setitem__("skills", "/tmp/untrusted-skills")),
            ("unknown field", "codexy", lambda data: data.__setitem__("unexpected", True)),
        )
        for label, component, mutate in cases:
            with self.subTest(label), tempfile.TemporaryDirectory() as temporary:
                root, core, github = activation_fixture(Path(temporary))
                manifest = (core if component == "codexy" else github) / ".codex-plugin/plugin.json"
                contents = json.loads(manifest.read_text(encoding="utf-8"))
                mutate(contents)
                manifest.write_text(json.dumps(contents), encoding="utf-8")
                self._assert_rollback(root, core, github)

    def test_duplicate_manifest_keys_fail_and_roll_back(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root, core, github = activation_fixture(Path(temporary))
            manifest = core / ".codex-plugin/plugin.json"
            manifest.write_text(
                manifest.read_text(encoding="utf-8").replace(
                    '"name": "codexy",', '"name": "codexy",\n  "name": "codexy",', 1,
                ), encoding="utf-8",
            )
            self._assert_rollback(root, core, github)

    def _assert_rollback(self, root: Path, core: Path, github: Path) -> None:
        calls: list[tuple[str, ...]] = []
        list_calls = 0

        def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
            nonlocal list_calls
            calls.append(tuple(command))
            if command[1:4] == ["plugin", "marketplace", "list"]:
                payload: object = {"marketplaces": [marketplace_entry(root / "marketplace")]}
            elif command[1:3] == ["plugin", "list"]:
                list_calls += 1
                payload = {"installed": [] if list_calls == 1 else [installed(core, "codexy"), installed(github, "codexy-github")]}
            else:
                payload = {"ok": True}
            return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

        home = root / "fresh Codex home"
        with self.assertRaisesRegex(ValueError, "component manifest"):
            run_github_pre_session(home, codex=executable(root), runner=runner, package_version="1.3.0")
        self.assertFalse((home / "agents").exists())
        self.assertEqual([call[1:4] for call in calls[-2:]], [
            ("plugin", "remove", "codexy-github@codexy"), ("plugin", "remove", "codexy@codexy"),
        ])


def copy_plugin(marketplace: Path, name: str) -> Path:
    destination = marketplace / "plugins" / name
    shutil.copytree(REPOSITORY / "plugins" / name, destination)
    return destination


def activation_fixture(root: Path) -> tuple[Path, Path, Path]:
    marketplace = root / "marketplace"
    return root, copy_plugin(marketplace, "codexy"), copy_plugin(marketplace, "codexy-github")


def executable(root: Path) -> Path:
    path = root / "trusted/codex"
    path.parent.mkdir(parents=True)
    path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    path.chmod(0o700)
    return path.resolve()


def marketplace_entry(root: Path) -> dict[str, object]:
    return {"name": "codexy", "root": str(root), "marketplaceSource": {"sourceType": "git", "source": OFFICIAL}}


def installed(root: Path, name: str) -> dict[str, object]:
    return {"pluginId": f"{name}@codexy", "name": name, "marketplaceName": "codexy", "version": "1.3.0", "installed": True, "enabled": True, "source": {"source": "local", "path": str(root)}, "marketplaceSource": {"sourceType": "git", "source": OFFICIAL}}


if __name__ == "__main__":
    unittest.main()
