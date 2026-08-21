"""Rollback cases for GitHub pre-session activation."""

import json
import subprocess
import tempfile
from pathlib import Path

from codexy_runtime_tools.github_pre_session import run_github_pre_session
from codexy_runtime_tools.updater import SyncResult
from github_pre_session_install_support import (
    executable,
    installed,
    marketplace,
    plugin,
    result,
    version,
)


class GithubPreSessionRollbackCases:
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
                    package_version=version(core),
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
