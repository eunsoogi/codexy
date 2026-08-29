from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.pre_session import run_pre_session
from codexy_runtime_tools.updater import SyncResult

try:
    from .pre_session_support import installed, make_plugin, marketplace
except ImportError:
    from pre_session_support import installed, make_plugin, marketplace


class PreSessionMarketplaceRepinTests(unittest.TestCase):
    def test_existing_official_marketplace_is_repinned_before_refresh(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marketplace_root = root / "marketplace"
            plugin = make_plugin(marketplace_root / "plugins/codexy")
            home = root / "home/.codex"
            home.mkdir(parents=True)
            (home / "config.toml").write_text(
                '[plugin_marketplaces.codexy]\nref = "main"\n', encoding="utf-8"
            )
            calls: list[tuple[str, ...]] = []
            target_version = "1.2.2"
            codex = "/trusted/codex"
            add = (
                codex,
                "plugin",
                "marketplace",
                "add",
                "eunsoogi/codexy",
                "--ref",
                f"v{target_version}",
                "--json",
            )

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                calls.append(tuple(command))
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    payload: object = {"marketplaces": [marketplace(marketplace_root)]}
                elif command[1:4] == ["plugin", "marketplace", "add"]:
                    ref = command[command.index("--ref") + 1]
                    (home / "config.toml").write_text(
                        f'[plugin_marketplaces.codexy]\nref = "{ref}"\n',
                        encoding="utf-8",
                    )
                    payload = {"ok": True}
                elif command[1:3] == ["plugin", "list"]:
                    payload = {
                        "installed": []
                        if calls.count(tuple(command)) == 1
                        else [installed(plugin)]
                    }
                else:
                    payload = {"ok": True}
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            result = run_pre_session(
                home,
                codex=Path(codex),
                runner=runner,
                synchronize=_ready,
                package_version=target_version,
            )
            self.assertEqual(result.version, target_version)
            self.assertEqual(
                calls,
                [
                    (codex, "plugin", "marketplace", "list", "--json"),
                    (codex, "plugin", "list", "--json"),
                    (codex, "plugin", "marketplace", "remove", "codexy", "--json"),
                    add,
                    (codex, "plugin", "marketplace", "list", "--json"),
                    (codex, "plugin", "marketplace", "upgrade", "codexy", "--json"),
                    (codex, "plugin", "marketplace", "list", "--json"),
                    (codex, "plugin", "add", "codexy@codexy", "--json"),
                    (codex, "plugin", "list", "--json"),
                ],
            )

    def test_failed_repin_restores_the_exact_prior_registration(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / "home/.codex"
            home.mkdir(parents=True)
            (home / "config.toml").write_text(
                '[plugin_marketplaces.codexy]\nref = "v1.1.0"\n', encoding="utf-8"
            )
            marketplace_root = root / "marketplace"
            make_plugin(marketplace_root / "plugins/codexy")
            state = {"registered": True, "ref": "v1.1.0"}

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    payload: object = {
                        "marketplaces": [marketplace(marketplace_root)]
                        if state["registered"]
                        else []
                    }
                    return subprocess.CompletedProcess(
                        command, 0, json.dumps(payload), ""
                    )
                if command[1:4] == ["plugin", "marketplace", "remove"]:
                    state["registered"] = False
                    return subprocess.CompletedProcess(command, 0, '{"ok":true}', "")
                if command[1:4] == ["plugin", "marketplace", "add"]:
                    ref = command[command.index("--ref") + 1]
                    if ref == "v1.2.2":
                        return subprocess.CompletedProcess(command, 1, "", "injected")
                    state.update(registered=True, ref=ref)
                    return subprocess.CompletedProcess(command, 0, '{"ok":true}', "")
                return subprocess.CompletedProcess(command, 0, '{"ok":true}', "")

            with self.assertRaisesRegex(RuntimeError, "marketplace add failed"):
                run_pre_session(
                    home,
                    codex=Path("/trusted/codex"),
                    runner=runner,
                    synchronize=_ready,
                    package_version="1.2.2",
                )
            self.assertEqual(state, {"registered": True, "ref": "v1.1.0"})

    def test_failed_post_add_verification_restores_the_exact_config_snapshot(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / "home/.codex"
            home.mkdir(parents=True)
            snapshot = (
                '[marketplaces.other]\nref = "wrong-ref"\n\n'
                '[marketplaces.codexy]\nsource = "https://github.com/eunsoogi/codexy.git"\n'
                'ref = "v1.1.0"\n'
            ).encode()
            (home / "config.toml").write_bytes(snapshot)
            marketplace_root = root / "marketplace"
            make_plugin(marketplace_root / "plugins/codexy")
            state = {"registered": True, "ref": "v1.1.0", "lists_after_target": 0}
            adds: list[str] = []

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    if state["ref"] == "v1.2.2":
                        state["lists_after_target"] += 1
                        return subprocess.CompletedProcess(command, 0, "not-json", "")
                    payload: object = {
                        "marketplaces": [marketplace(marketplace_root)]
                        if state["registered"]
                        else []
                    }
                    return subprocess.CompletedProcess(
                        command, 0, json.dumps(payload), ""
                    )
                if command[1:4] == ["plugin", "marketplace", "remove"]:
                    state["registered"] = False
                elif command[1:4] == ["plugin", "marketplace", "add"]:
                    ref = command[command.index("--ref") + 1]
                    adds.append(ref)
                    state.update(registered=True, ref=ref)
                return subprocess.CompletedProcess(command, 0, '{"ok":true}', "")

            with self.assertRaisesRegex(
                ValueError, "marketplace list returned invalid JSON"
            ):
                run_pre_session(
                    home,
                    codex=Path("/trusted/codex"),
                    runner=runner,
                    synchronize=_ready,
                    package_version="1.2.2",
                )
            self.assertEqual(adds, ["v1.2.2", "v1.1.0"])
            self.assertEqual(state["ref"], "v1.1.0")
            self.assertGreater(state["lists_after_target"], 0)
            self.assertEqual((home / "config.toml").read_bytes(), snapshot)


def _ready(plugin: Path, home: Path, mode: str) -> SyncResult:
    return SyncResult(mode, "ready", "codexy", str(plugin), str(home), False, False, ())


if __name__ == "__main__":
    unittest.main()
