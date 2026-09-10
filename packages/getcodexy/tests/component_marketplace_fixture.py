"""Marketplace materialization helpers for lifecycle fixtures."""

import os
import shutil
from pathlib import Path

from packages.getcodexy.tests.component_watcher_fixture import install_watcher_runtime


def populate_plugins(marketplace: Path) -> None:
    repository = Path(__file__).resolve().parents[3]
    for plugin in ("codexy", "codexy-github", "codexy-devtools"):
        destination = marketplace / "plugins" / plugin
        if not destination.exists():
            shutil.copytree(repository / "plugins" / plugin, destination)
        if plugin == "codexy" and os.name == "nt":
            install_watcher_runtime(destination)
