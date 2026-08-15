from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.github_pre_session import run_github_pre_session
from codexy_runtime_tools.updater import SyncResult


REPOSITORY = Path(__file__).resolve().parents[3]
OFFICIAL = "https://github.com/eunsoogi/codexy.git"


class GithubPreSessionMarketplaceTests(unittest.TestCase):
    def test_fresh_home_registers_and_rereads_the_official_marketplace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            market = root / "marketplace"
            core = copy_plugin(market, "codexy")
            github = copy_plugin(market, "codexy-github")
            codex = executable(root)
            calls: list[tuple[str, ...]] = []
            marketplace_registered = False
            plugin_adds = 0

            def runner(command: list[str]) -> subprocess.CompletedProcess[str]:
                nonlocal marketplace_registered, plugin_adds
                calls.append(tuple(command))
                if command[1:4] == ["plugin", "marketplace", "list"]:
                    payload: object = {
                        "marketplaces": [marketplace(market)]
                        if marketplace_registered
                        else []
                    }
                elif command[1:4] == ["plugin", "marketplace", "add"]:
                    marketplace_registered = True
                    payload = {"ok": True}
                elif command[1:3] == ["plugin", "list"]:
                    payload = {
                        "installed": []
                        if plugin_adds == 0
                        else [
                            installed(core, "codexy"),
                            installed(github, "codexy-github"),
                        ]
                    }
                else:
                    plugin_adds += 1
                    payload = {"ok": True}
                return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

            run_github_pre_session(
                root / "fresh Codex home",
                codex=codex,
                runner=runner,
                synchronize=lambda _root, home, mode: sync_result(mode, home),
                activate_github=lambda *_: True,
            )

            self.assertEqual(
                calls[:4],
                [
                    (str(codex), "plugin", "marketplace", "list", "--json"),
                    (
                        str(codex),
                        "plugin",
                        "marketplace",
                        "add",
                        "eunsoogi/codexy",
                        "--ref",
                        "v1.3.0",
                        "--json",
                    ),
                    (str(codex), "plugin", "marketplace", "list", "--json"),
                    (str(codex), "plugin", "list", "--json"),
                ],
            )


def copy_plugin(marketplace_root: Path, name: str) -> Path:
    destination = marketplace_root / "plugins" / name
    shutil.copytree(REPOSITORY / "plugins" / name, destination)
    return destination


def executable(root: Path) -> Path:
    path = root / "trusted/codex"
    path.parent.mkdir(parents=True)
    path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    path.chmod(0o700)
    return path.resolve()


def marketplace(root: Path) -> dict[str, object]:
    return {
        "name": "codexy",
        "root": str(root),
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def installed(root: Path, name: str) -> dict[str, object]:
    return {
        "pluginId": f"{name}@codexy",
        "name": name,
        "marketplaceName": "codexy",
        "version": "1.3.0",
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": str(root)},
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def sync_result(mode: str, home: Path) -> SyncResult:
    status = "update_required" if mode == "check" else "completed"
    return SyncResult(
        mode,
        status,
        "codexy",
        "test",
        str(home),
        mode == "install",
        mode == "install",
        (),
    )


if __name__ == "__main__":
    unittest.main()
