"""Fixtures for verified default activation and legacy path-safety cases."""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_candidate_package,
    configure_legacy_core_package,
)


REPOSITORY = Path(__file__).resolve().parents[3]
OFFICIAL = "https://github.com/eunsoogi/codexy.git"


def copy_plugin(marketplace: Path, name: str) -> Path:
    destination = marketplace / "plugins" / name
    shutil.copytree(REPOSITORY / "plugins" / name, destination)
    configure_candidate_package(destination)
    return destination


def activation_fixture(
    root: Path, *, legacy_core: bool = False
) -> tuple[Path, Path, Path]:
    """Build current packages, optionally with the historical core MCP contract."""
    marketplace = root / "marketplace"
    core = copy_plugin(marketplace, "codexy")
    if legacy_core:
        # The retained symlink guard applies to registered legacy MCP packages.
        configure_legacy_core_package(core, version(core))
    return root, core, copy_plugin(marketplace, "codexy-github")


def executable(root: Path) -> Path:
    path = root / "trusted/codex"
    path.parent.mkdir(parents=True)
    path.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    path.chmod(0o700)
    return path.resolve()


def marketplace_entry(root: Path) -> dict[str, object]:
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
        "version": version(root),
        "installed": True,
        "enabled": True,
        "source": {"source": "local", "path": str(root)},
        "marketplaceSource": {"sourceType": "git", "source": OFFICIAL},
    }


def version(root: Path) -> str:
    manifest = root / ".codex-plugin/plugin.json"
    return str(json.loads(manifest.read_text(encoding="utf-8"))["version"])
