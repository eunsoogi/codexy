"""Fixtures for GitHub pre-session installation cases."""

import json
import shutil
from pathlib import Path

from codexy_runtime_tools.updater import SyncResult

OFFICIAL = "https://github.com/eunsoogi/codexy.git"
REPOSITORY = Path(__file__).resolve().parents[3]


def marketplace(root: Path) -> dict[str, object]:
    return {
        "name": "codexy",
        "root": str(root),
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def plugin(root: Path, name: str) -> Path:
    shutil.copytree(REPOSITORY / "plugins" / name, root)
    return root


def executable(root: Path) -> Path:
    path = root / "trusted/codex"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    path.chmod(0o700)
    return path.resolve()


def result(mode: str, status: str, home: Path, changed: bool) -> SyncResult:
    return SyncResult(mode, status, "codexy", "test", str(home), changed, changed, ())


def installed(root: Path, name: str) -> dict[str, object]:
    return {
        "pluginId": f"{name}@codexy",
        "name": name,
        "marketplaceName": "codexy",
        "version": version(root),
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": str(root)},
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def version(root: Path) -> str:
    manifest = root / ".codex-plugin/plugin.json"
    return str(json.loads(manifest.read_text(encoding="utf-8"))["version"])


def disabled(root: Path, name: str) -> dict[str, object]:
    value = installed(root, name)
    value["enabled"] = False
    return value
