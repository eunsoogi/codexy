from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.component_integrity import frozen_component
from codexy_runtime_tools.github_pre_session import (
    run_github_pre_session,
    trusted_codex,
)
from codexy_runtime_tools.updater import SyncResult
from github_pre_session_install_support import (
    OFFICIAL,
    REPOSITORY,
    disabled,
    executable,
    installed,
    marketplace,
    plugin,
    result,
    version,
)
from github_pre_session_rollback_cases import GithubPreSessionRollbackCases


class GithubPreSessionInstallTests(GithubPreSessionRollbackCases, unittest.TestCase):
    def test_requires_an_absolute_host_executable(self) -> None:
        with self.assertRaisesRegex(ValueError, "absolute path"):
            trusted_codex(Path("codex"))

    def test_freezes_verified_component_before_cache_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            component = plugin(Path(temporary) / "codexy-github", "codexy-github")
            with frozen_component(component, "codexy-github") as frozen:
                original = (frozen / "agents/codexy-weaver.toml").read_bytes()
                (component / "agents/codexy-weaver.toml").write_text(
                    "replaced", encoding="utf-8"
                )
                self.assertEqual(
                    (frozen / "agents/codexy-weaver.toml").read_bytes(), original
                )

    def test_rejects_a_symlinked_component_ancestor(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            component = plugin(Path(temporary) / "codexy-github", "codexy-github")
            agents = component / "agents"
            moved = component / "trusted-agents"
            agents.rename(moved)
            agents.symlink_to(moved, target_is_directory=True)
            with self.assertRaisesRegex(
                (OSError, ValueError), "Too many levels|Not a directory|link|reparse"
            ):
                with frozen_component(component, "codexy-github"):
                    pass

    def test_github_install_resolves_core_then_activates_both_packages(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / "home/.codex"
            home.mkdir(parents=True)
            market = root / "marketplace"
            core = plugin(market / "plugins/codexy", "codexy")
            github = plugin(market / "plugins/codexy-github", "codexy-github")
            codex = executable(root)
            calls: list[tuple[str, ...]] = []
            synchronized: list[tuple[str, Path]] = []

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                calls.append(tuple(command))
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    payload: object = {"marketplaces": [marketplace(market)]}
                elif command[1:3] == ["plugin", "list"]:
                    payload = {
                        "installed": []
                        if len(calls) == 2
                        else [
                            installed(core, "codexy"),
                            installed(github, "codexy-github"),
                        ]
                    }
                else:
                    payload = {"ok": True}
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            def synchronize(plugin_root: Path, home: Path, mode: str) -> SyncResult:
                synchronized.append((plugin_root, home))
                return SyncResult(
                    mode,
                    "update_required" if mode == "check" else "completed",
                    "codexy",
                    str(plugin_root),
                    str(home),
                    mode == "install",
                    mode == "install",
                    (),
                )

            result = run_github_pre_session(
                home,
                codex=codex,
                runner=runner,
                synchronize=synchronize,
                activate_github=lambda plugin_root, home: synchronized.append(
                    (plugin_root, home)
                ),
                package_version=version(core),
            )

            self.assertTrue(result.core_root.samefile(core))
            self.assertTrue(result.github_root.samefile(github))
            self.assertTrue(result.changed)
            self.assertEqual(
                calls,
                [
                    (str(codex), "plugin", "marketplace", "list", "--json"),
                    (str(codex), "plugin", "list", "--json"),
                    (str(codex), "plugin", "add", "codexy@codexy", "--json"),
                    (str(codex), "plugin", "add", "codexy-github@codexy", "--json"),
                    (str(codex), "plugin", "list", "--json"),
                ],
            )
            home_alias = root / "home-alias"
            home_alias.symlink_to(home.parent, target_is_directory=True)
            self.assertEqual(len(synchronized), 3)
            expected_plugin_roots = [
                synchronized[0][0],
                synchronized[0][0],
                synchronized[2][0],
            ]
            expected = [(path, home_alias / ".codex") for path in expected_plugin_roots]
            self.assertEqual(
                [path for path, _ in synchronized],
                [path for path, _ in expected],
            )
            for (_, actual_home), (_, expected_home) in zip(synchronized, expected):
                self.assertTrue(actual_home.samefile(expected_home))
            self.assertNotIn(core.resolve(), [path for path, _ in synchronized])
            self.assertNotIn(github.resolve(), [path for path, _ in synchronized])

    def test_rejects_tampered_cache_before_agent_activation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            market = root / "marketplace"
            core = plugin(market / "plugins/codexy", "codexy")
            github = plugin(market / "plugins/codexy-github", "codexy-github")
            (github / "agents/catalog.toml").write_text("tampered", encoding="utf-8")
            invoked: list[tuple[str, ...]] = []

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                invoked.append(tuple(command))
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    payload: object = {"marketplaces": [marketplace(market)]}
                elif command[1:3] == ["plugin", "list"]:
                    payload = {
                        "installed": []
                        if len(invoked) == 2
                        else [
                            installed(core, "codexy"),
                            installed(github, "codexy-github"),
                        ]
                    }
                else:
                    payload = {"ok": True}
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            with self.assertRaisesRegex(ValueError, "component registration"):
                run_github_pre_session(
                    root / "home/.codex",
                    codex=executable(root),
                    runner=runner,
                    synchronize=lambda *_: self.fail(
                        "tampered cache reached core activation"
                    ),
                    activate_github=lambda *_: self.fail(
                        "tampered cache reached GitHub activation"
                    ),
                    package_version=version(core),
                )
            self.assertEqual(
                invoked[-2:],
                [
                    (
                        str(executable(root)),
                        "plugin",
                        "remove",
                        "codexy-github@codexy",
                        "--json",
                    ),
                    (
                        str(executable(root)),
                        "plugin",
                        "remove",
                        "codexy@codexy",
                        "--json",
                    ),
                ],
            )

    def test_rejects_disabled_component_before_mutating_host_state(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            market = root / "marketplace"
            core = plugin(market / "plugins/codexy", "codexy")
            calls: list[tuple[str, ...]] = []

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                calls.append(tuple(command))
                payload = (
                    {"marketplaces": [marketplace(market)]}
                    if command[1:4] == ["plugin", "marketplace", "list"]
                    else {"installed": [disabled(core, "codexy")]}
                )
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            with self.assertRaisesRegex(ValueError, "disabled codexy"):
                run_github_pre_session(
                    root / "home/.codex",
                    codex=executable(root),
                    runner=runner,
                    package_version=version(core),
                )
            self.assertEqual(len(calls), 2)


if __name__ == "__main__":
    unittest.main()
