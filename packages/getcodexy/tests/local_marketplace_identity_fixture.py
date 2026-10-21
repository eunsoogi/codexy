"""Local marketplace host double used by installation and identity tests."""

from __future__ import annotations

import json
import shutil
import stat
import subprocess
import tempfile
from pathlib import Path

from component_lifecycle_support import HOOK_LIST_HOST
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_core_package,
)

REPOSITORY = Path(__file__).resolve().parents[3]
MANIFEST = load_component_manifest()


class LocalHost:
    """Model a local archive source while recording host reads and mutations."""

    def __init__(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.home = self.root / "codex-home"
        self.marketplace = self.root / "candidate-marketplace"
        self.home.mkdir()
        self._materialize_archive()
        self.codex = self.root / "trusted-codex"
        self.codex.write_text(HOOK_LIST_HOST, encoding="utf-8")
        self.codex.chmod(self.codex.stat().st_mode | stat.S_IXUSR)
        self.selection, self.mutations = set(), []
        self.marketplace_reads = 0
        self.installed_source: dict[str, str] | None = None

    def __enter__(self) -> "LocalHost":
        return self

    def __exit__(self, *_: object) -> None:
        self.temporary.cleanup()

    def operate(self, command: str, identifier: str, requested: tuple[str, ...] = ()):
        return run_operation(
            command,
            requested,
            self.home,
            self.codex,
            self.run,
            operation_id=identifier,
        )

    def _materialize_archive(self) -> None:
        source = REPOSITORY / ".agents/plugins/marketplace.json"
        destination = self.marketplace / ".agents/plugins/marketplace.json"
        destination.parent.mkdir(parents=True)
        shutil.copy2(source, destination)
        for name in (component.plugin for component in MANIFEST.components):
            shutil.copytree(
                REPOSITORY / "plugins" / name,
                self.marketplace / "plugins" / name,
            )
        configure_core_package(self.marketplace / "plugins/codexy", MANIFEST.version)

    def run(self, command: list[str]) -> subprocess.CompletedProcess[str]:
        tail = tuple(command[1:])
        if tail == ("plugin", "marketplace", "list", "--json"):
            self.marketplace_reads += 1
            payload: object = {
                "marketplaces": [
                    {
                        "name": "codexy",
                        "root": str(self.marketplace),
                        "marketplaceSource": {
                            "sourceType": "local",
                            "source": str(self.marketplace),
                        },
                    }
                ]
            }
        elif tail == ("plugin", "list", "--json"):
            payload = {
                "installed": [
                    self._installed(name, self.installed_source)
                    for name in (component.plugin for component in MANIFEST.components)
                    if self._component(name) in self.selection
                ]
            }
        elif tail[:2] == ("plugin", "add"):
            name = tail[2].split("@", 1)[0]
            self.selection.add(self._component(name))
            self.mutations.append(tail)
            payload = {"ok": True}
        else:
            self.mutations.append(tail)
            payload = {"ok": True}
        return subprocess.CompletedProcess(command, 0, json.dumps(payload), "")

    def _installed(
        self, name: str, installed_source: dict[str, str] | None
    ) -> dict[str, object]:
        plugin = self.marketplace / "plugins" / name
        return {
            "pluginId": f"{name}@codexy",
            "name": name,
            "marketplaceName": "codexy",
            "version": MANIFEST.version,
            "installed": True,
            "enabled": True,
            "source": {"source": "local", "path": str(plugin)},
            "marketplaceSource": installed_source
            or {
                "sourceType": "local",
                "source": str(self.marketplace),
            },
        }

    @staticmethod
    def _component(name: str) -> str:
        return {
            "codexy": "core",
            "codexy-github": "github",
            "codexy-devtools": "devtools",
        }[name]
