"""Marketplace materialization helpers for lifecycle fixtures."""

import shutil
from pathlib import Path


def populate_plugins(marketplace: Path) -> None:
    repository = Path(__file__).resolve().parents[3]
    for plugin in ("codexy", "codexy-github", "codexy-devtools"):
        destination = marketplace / "plugins" / plugin
        if not destination.exists():
            shutil.copytree(repository / "plugins" / plugin, destination)
