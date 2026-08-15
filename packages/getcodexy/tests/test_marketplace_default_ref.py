from __future__ import annotations

import base64
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


class MarketplaceDefaultRefTests(unittest.TestCase):
    def test_legacy_default_ref_is_pinned_before_activation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home, snapshot = _unsafe_home(root, None)
            marketplace_root = root / "marketplace"
            plugin = make_plugin(marketplace_root / "plugins/codexy")
            state = {"registered": True, "ref": None, "installed": False}

            result = run_pre_session(
                home,
                codex=Path("/trusted/codex"),
                runner=_host(home, marketplace_root, state),
                synchronize=_ready,
                package_version="1.2.2",
            )

            self.assertEqual(result.version, "1.2.2")
            self.assertEqual(
                state, {"registered": True, "ref": "v1.2.2", "installed": True}
            )
            self.assertIn('ref = "v1.2.2"', (home / "config.toml").read_text())
            self.assertFalse((home / "getcodexy/marketplace-recovery.json").exists())
            self.assertNotEqual((home / "config.toml").read_bytes(), snapshot)
            self.assertTrue(plugin.is_dir())

    def test_failed_legacy_default_ref_repin_quarantines_and_records_recovery(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home, snapshot = _unsafe_home(root, None)
            marketplace_root = root / "marketplace"
            make_plugin(marketplace_root / "plugins/codexy")
            state = {"registered": True, "ref": None, "installed": False}

            with self.assertRaisesRegex(
                RuntimeError, "unsafe marketplace registration was removed"
            ):
                run_pre_session(
                    home,
                    codex=Path("/trusted/codex"),
                    runner=_host(home, marketplace_root, state, fail_target=True),
                    synchronize=_ready,
                    package_version="1.2.2",
                )

            self.assertEqual(
                state, {"registered": False, "ref": None, "installed": False}
            )
            self.assertNotIn(
                "[marketplaces.codexy]", (home / "config.toml").read_text()
            )
            receipt = json.loads(
                (home / "getcodexy/marketplace-recovery.json").read_text()
            )
            self.assertEqual(receipt["schema"], "getcodexy.marketplace-recovery.v1")
            self.assertEqual(receipt["reason"], "unsafe-default-ref")
            self.assertEqual(base64.b64decode(receipt["config_toml_base64"]), snapshot)

    def test_failed_main_ref_repin_is_also_quarantined(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home, snapshot = _unsafe_home(root, "main")
            marketplace_root = root / "marketplace"
            make_plugin(marketplace_root / "plugins/codexy")
            state = {"registered": True, "ref": "main", "installed": False}

            with self.assertRaisesRegex(
                RuntimeError, "unsafe marketplace registration was removed"
            ):
                run_pre_session(
                    home,
                    codex=Path("/trusted/codex"),
                    runner=_host(home, marketplace_root, state, fail_target=True),
                    synchronize=_ready,
                    package_version="1.2.2",
                )

            self.assertEqual(
                state, {"registered": False, "ref": None, "installed": False}
            )
            self.assertNotIn(
                "[marketplaces.codexy]", (home / "config.toml").read_text()
            )
            receipt = json.loads(
                (home / "getcodexy/marketplace-recovery.json").read_text()
            )
            self.assertEqual(receipt["reason"], "unsafe-main-ref")
            self.assertEqual(base64.b64decode(receipt["config_toml_base64"]), snapshot)


def _unsafe_home(root: Path, ref: str | None) -> tuple[Path, bytes]:
    home = root / "home/.codex"
    home.mkdir(parents=True)
    ref_line = b"" if ref is None else f'ref = "{ref}"\n'.encode()
    snapshot = b'[marketplaces.other]\nref = "v0.1.0"\n\n' + (
        b'[marketplaces.codexy]\nsource = "https://github.com/eunsoogi/codexy.git"\n'
        + ref_line
    )
    (home / "config.toml").write_bytes(snapshot)
    return home, snapshot


def _host(
    home: Path,
    marketplace_root: Path,
    state: dict[str, object],
    *,
    fail_target: bool = False,
):
    def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
        if command[1:4] == ["plugin", "marketplace", "list"]:
            payload: object = {
                "marketplaces": [marketplace(marketplace_root)]
                if state["registered"]
                else []
            }
        elif command[1:4] == ["plugin", "marketplace", "remove"]:
            state.update(registered=False, ref=None)
            payload = {"ok": True}
        elif command[1:4] == ["plugin", "marketplace", "add"]:
            ref = command[command.index("--ref") + 1]
            if fail_target and ref == "v1.2.2":
                return subprocess.CompletedProcess(command, 1, "", "injected")
            state.update(registered=True, ref=ref)
            (home / "config.toml").write_text(
                f'[marketplaces.codexy]\nref = "{ref}"\n', encoding="utf-8"
            )
            payload = {"ok": True}
        elif command[1:3] == ["plugin", "list"]:
            payload = {
                "installed": [installed(marketplace_root / "plugins/codexy")]
                if state["installed"]
                else []
            }
        elif command[1:3] == ["plugin", "add"]:
            state["installed"] = True
            payload = {"ok": True}
        else:
            payload = {"ok": True}
        return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

    return runner


def _ready(*_: object) -> SyncResult:
    return SyncResult("check", "ready", "codexy", "", "", False, False, ())
