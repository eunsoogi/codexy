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
)


class GithubPreSessionInstallTests(unittest.TestCase):
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
                root / "home/.codex",
                codex=codex,
                runner=runner,
                synchronize=synchronize,
                activate_github=lambda plugin_root, home: synchronized.append(
                    (plugin_root, home)
                ),
                package_version="1.3.0",
            )

            self.assertEqual(result.core_root, core.resolve())
            self.assertEqual(result.github_root, github.resolve())
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
            self.assertEqual(
                synchronized,
                [
                    *[
                        (path, (root / "home/.codex").resolve())
                        for path, _ in synchronized
                    ],
                ],
            )
            self.assertNotIn(core.resolve(), [path for path, _ in synchronized])
            self.assertNotIn(github.resolve(), [path for path, _ in synchronized])

    def test_rejects_tampered_cache_before_agent_activation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            market = root / "marketplace"
            core = plugin(market / "plugins/codexy", "codexy")
            github = plugin(market / "plugins/codexy-github", "codexy-github")
            (github / "agents/codexy-weaver.toml").write_text(
                "tampered", encoding="utf-8"
            )
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

            with self.assertRaisesRegex(ValueError, "component integrity mismatch"):
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
                    package_version="1.3.0",
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
                    package_version="1.3.0",
                )
            self.assertEqual(len(calls), 2)

    def test_rolls_back_agent_and_plugin_state_when_github_activation_fails(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            market = root / "marketplace"
            core = plugin(market / "plugins/codexy", "codexy")
            github = plugin(market / "plugins/codexy-github", "codexy-github")
            home = root / "home/.codex"
            home.mkdir(parents=True)
            (home / "config.toml").write_text("original = true\n", encoding="utf-8")
            calls: list[tuple[str, ...]] = []

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

            def synchronize(_root: Path, target: Path, mode: str) -> SyncResult:
                if mode == "check":
                    return result(mode, "update_required", target, False)
                (target / "config.toml").write_text(
                    "mutated = true\n", encoding="utf-8"
                )
                (target / "agents/codexy").mkdir(parents=True)
                (target / "agents/codexy/core.toml").write_text(
                    "core", encoding="utf-8"
                )
                return result(mode, "completed", target, True)

            with self.assertRaisesRegex(RuntimeError, "activation failed"):
                run_github_pre_session(
                    home,
                    codex=executable(root),
                    runner=runner,
                    synchronize=synchronize,
                    activate_github=lambda *_: (_ for _ in ()).throw(
                        RuntimeError("activation failed")
                    ),
                    package_version="1.3.0",
                )
            self.assertEqual(
                (home / "config.toml").read_text(encoding="utf-8"), "original = true\n"
            )
            self.assertFalse((home / "agents/codexy").exists())
            self.assertEqual(
                calls[-2:],
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


if __name__ == "__main__":
    unittest.main()
