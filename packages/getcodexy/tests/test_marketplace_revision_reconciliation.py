from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.pre_session import run_pre_session
from codexy_runtime_tools.updater import SyncResult


OFFICIAL = "https://github.com/eunsoogi/codexy.git"
VERSION = "1.2.2"
TAG = f"v{VERSION}"


class MarketplaceRevisionReconciliationTests(unittest.TestCase):
    def test_correct_tag_pin_survives_upgrade_reload(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root, home, state = _fixture(Path(temporary), None)

            result = run_pre_session(
                home,
                codex=Path("/trusted/codex"),
                runner=_runner(home, root, state),
                synchronize=_ready,
                package_version=VERSION,
            )

            self.assertEqual(result.version, VERSION)
            self.assertTrue(state["installed"])
            self.assertEqual(
                _metadata(root), {"ref_name": TAG, "revision": state["tag"]}
            )
            self.assertIn(f'ref = "{TAG}"', (home / "config.toml").read_text())
            self.assertEqual(_git(root, "rev-parse", "HEAD"), state["tag"])

    def test_upgrade_reload_drift_is_quarantined_before_plugin_activation(self) -> None:
        cases = {
            "null-ref": (None, None),
            "main-ref": ("main", "main"),
            "revision": (TAG, "main"),
        }
        for name, (config_ref, metadata_ref) in cases.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                root, home, state = _fixture(
                    Path(temporary), (config_ref, metadata_ref)
                )

                with self.assertRaisesRegex(RuntimeError, "marketplace.*quarantined"):
                    run_pre_session(
                        home,
                        codex=Path("/trusted/codex"),
                        runner=_runner(home, root, state),
                        synchronize=_ready,
                        package_version=VERSION,
                    )

                self.assertFalse(state["registered"])
                self.assertFalse(state["installed"])
                self.assertNotIn(
                    "[marketplaces.codexy]", (home / "config.toml").read_text()
                )
                receipt = json.loads(
                    (home / "getcodexy/marketplace-recovery.json").read_text()
                )
                self.assertEqual(receipt["reason"], "post-upgrade-marketplace-drift")


def _fixture(
    root: Path, drift: tuple[str | None, str | None] | None
) -> tuple[Path, Path, dict[str, object]]:
    marketplace = root / "marketplace"
    plugin = marketplace / "plugins/codexy/.codex-plugin/plugin.json"
    plugin.parent.mkdir(parents=True)
    plugin.write_text(
        json.dumps(
            {
                "name": "codexy",
                "repository": "https://github.com/eunsoogi/codexy",
                "version": VERSION,
            }
        ),
        encoding="utf-8",
    )
    _git(marketplace, "init", "-q")
    _git(marketplace, "branch", "-M", "main")
    _git(marketplace, "config", "user.name", "fixture")
    _git(marketplace, "config", "user.email", "fixture@example.invalid")
    _git(marketplace, "add", ".")
    _git(marketplace, "commit", "-qm", "fixture main")
    main_revision = _git(marketplace, "rev-parse", "HEAD")
    (marketplace / "release-marker").write_text("tag", encoding="utf-8")
    _git(marketplace, "add", "release-marker")
    _git(marketplace, "commit", "-qm", "fixture release")
    _git(marketplace, "tag", TAG)
    tag_revision = _git(marketplace, "rev-parse", f"{TAG}^{{commit}}")
    _git(marketplace, "checkout", "-q", "--detach", TAG)
    _write_metadata(marketplace, TAG, tag_revision)

    home = root / "home/.codex"
    home.mkdir(parents=True)
    (home / "config.toml").write_text(
        '[marketplaces.codexy]\nref = "main"\n', encoding="utf-8"
    )
    state: dict[str, object] = {
        "registered": True,
        "installed": False,
        "drift": drift,
        "main": main_revision,
        "tag": tag_revision,
    }
    return marketplace, home, state


def _runner(home: Path, root: Path, state: dict[str, object]):
    def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
        if command[1:4] == ["plugin", "marketplace", "list"]:
            payload: object = {
                "marketplaces": [_marketplace(root)] if state["registered"] else []
            }
        elif command[1:4] == ["plugin", "marketplace", "remove"]:
            state["registered"] = False
            payload = {"ok": True}
        elif command[1:4] == ["plugin", "marketplace", "add"]:
            state["registered"] = True
            _write_config(home, TAG)
            _git(root, "checkout", "-q", "--detach", TAG)
            _write_metadata(root, TAG, state["tag"])
            payload = {"ok": True}
        elif command[1:4] == ["plugin", "marketplace", "upgrade"]:
            drift = state["drift"]
            if drift is not None:
                config_ref, metadata_ref = drift
                _write_config(home, config_ref)
                _git(root, "checkout", "-q", "--detach", "main")
                _write_metadata(root, metadata_ref, state["main"])
            payload = {"ok": True}
        elif command[1:3] == ["plugin", "list"]:
            payload = {"installed": [_installed(root)] if state["installed"] else []}
        elif command[1:3] == ["plugin", "add"]:
            state["installed"] = True
            payload = {"ok": True}
        else:
            payload = {"ok": True}
        return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

    return runner


def _marketplace(root: Path) -> dict[str, object]:
    return {
        "name": "codexy",
        "root": str(root),
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def _installed(root: Path) -> dict[str, object]:
    return {
        "pluginId": "codexy@codexy",
        "name": "codexy",
        "marketplaceName": "codexy",
        "version": VERSION,
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": str(root / "plugins/codexy")},
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def _write_config(home: Path, ref: str | None) -> None:
    value = "[marketplaces.codexy]\n"
    if ref is not None:
        value += f'ref = "{ref}"\n'
    (home / "config.toml").write_text(value, encoding="utf-8")


def _write_metadata(root: Path, ref: str | None, revision: object) -> None:
    (root / ".codex-marketplace-install.json").write_text(
        json.dumps(
            {
                "ref_name": ref,
                "revision": revision,
                "source": OFFICIAL,
                "source_type": "git",
                "sparse_paths": [],
            }
        ),
        encoding="utf-8",
    )


def _metadata(root: Path) -> dict[str, object]:
    data = json.loads((root / ".codex-marketplace-install.json").read_text())
    return {"ref_name": data["ref_name"], "revision": data["revision"]}


def _git(root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout.strip()


def _ready(*_: object) -> SyncResult:
    return SyncResult("check", "ready", "codexy", "", "", False, False, ())


if __name__ == "__main__":
    unittest.main()
