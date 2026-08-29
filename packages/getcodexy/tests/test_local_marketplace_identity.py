from __future__ import annotations

import json
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor, status
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.plugin_resolution import marketplace_identity


REPOSITORY = Path(__file__).parents[3]
MANIFEST = load_component_manifest()
PLUGIN_NAMES = tuple(component.plugin for component in MANIFEST.components)


class LocalMarketplaceIdentityTests(unittest.TestCase):
    def test_captured_local_source_binds_to_the_reported_marketplace_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            identity = marketplace_identity(
                {
                    "marketplaces": [
                        {
                            "name": "codexy",
                            "root": str(root),
                            "marketplaceSource": {
                                "sourceType": "local",
                                "source": str(root),
                            },
                        }
                    ]
                }
            )

        self.assertEqual(identity.source_type, "local")
        self.assertEqual(identity.root, root)
        self.assertEqual(
            identity.host_source,
            {"sourceType": "local", "source": str(root)},
        )

    def test_local_archive_install_and_status_use_local_host_identity(self) -> None:
        with LocalHost() as host:
            receipt = run_operation(
                "install",
                (),
                host.home,
                host.codex,
                host.run,
                operation_id="op-local-archive",
            )
            observed = status(host.home, codex=host.codex, runner=host.run)
            diagnosed = doctor(host.home, codex=host.codex, runner=host.run)

        self.assertEqual(receipt["outcome"], "completed")
        self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
        self.assertEqual(
            observed["installed_components"], ["core", "github", "devtools"]
        )
        self.assertEqual(observed["errors"], [])
        self.assertEqual(diagnosed["inventory_consistency"], "consistent")
        self.assertNotIn({"code": "invalid-installed-inventory"}, diagnosed["errors"])

    def test_foreign_unbound_or_ambiguous_local_sources_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            foreign = root / "foreign"
            cases = (
                (
                    "foreign",
                    {
                        "sourceType": "git",
                        "source": "https://example.invalid/foreign.git",
                    },
                    "Codexy marketplace source",
                ),
                ("unbound", None, "marketplace source"),
            )
            for name, source, message in cases:
                with self.subTest(name=name):
                    entry = {"name": "codexy", "root": str(root)}
                    if source is not None:
                        entry["marketplaceSource"] = source
                    with self.assertRaisesRegex(ValueError, message):
                        marketplace_identity({"marketplaces": [entry]})

            local = {
                "name": "codexy",
                "root": str(root),
                "marketplaceSource": {
                    "sourceType": "local",
                    "source": str(root),
                },
            }
            with self.assertRaisesRegex(ValueError, "exactly one"):
                marketplace_identity({"marketplaces": [local, local]})

    def test_mismatched_installed_source_is_rejected_before_mutation(self) -> None:
        with LocalHost() as host:
            foreign = host.root / "foreign-archive"
            host.selection.add("core")
            host.installed_source = {
                "sourceType": "local",
                "source": str(foreign),
            }
            receipt = run_operation(
                "install",
                ("github",),
                host.home,
                host.codex,
                host.run,
                operation_id="op-mismatched-local-source",
            )

        self.assertEqual(receipt["outcome"], "rejected")
        self.assertEqual(host.mutations, [])

    def test_archive_provenance_mismatch_is_rejected_before_mutation(self) -> None:
        for field in ("source_path", "version", "manifest_repository"):
            with self.subTest(field=field), LocalHost() as host:
                metadata_path = host.marketplace / ".agents/plugins/marketplace.json"
                metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
                if field == "source_path":
                    metadata["plugins"][0]["name"] = "../foreign"
                    metadata["plugins"][0]["source"]["path"] = "./plugins/../foreign"
                elif field == "version":
                    metadata["plugins"][0]["version"] = "0.0.0"
                else:
                    manifest = (
                        host.marketplace / "plugins/codexy/.codex-plugin/plugin.json"
                    )
                    data = json.loads(manifest.read_text(encoding="utf-8"))
                    data["repository"] = "https://example.invalid/foreign"
                    manifest.write_text(json.dumps(data), encoding="utf-8")
                metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
                receipt = run_operation(
                    "install",
                    (),
                    host.home,
                    host.codex,
                    host.run,
                    operation_id=f"op-provenance-{field}",
                )

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(host.mutations, [])


class LocalHost:
    def __init__(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.home = self.root / "codex-home"
        self.marketplace = self.root / "candidate-marketplace"
        self.home.mkdir()
        self._materialize_archive()
        self.codex = self.root / "trusted-codex"
        self.codex.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        self.codex.chmod(self.codex.stat().st_mode | stat.S_IXUSR)
        self.selection: set[str] = set()
        self.mutations: list[tuple[str, ...]] = []
        self.installed_source: dict[str, str] | None = None

    def __enter__(self) -> "LocalHost":
        return self

    def __exit__(self, *_: object) -> None:
        self.temporary.cleanup()

    def _materialize_archive(self) -> None:
        source = REPOSITORY / ".agents/plugins/marketplace.json"
        destination = self.marketplace / ".agents/plugins/marketplace.json"
        destination.parent.mkdir(parents=True)
        shutil.copy2(source, destination)
        for name in PLUGIN_NAMES:
            shutil.copytree(
                REPOSITORY / "plugins" / name,
                self.marketplace / "plugins" / name,
            )

    def run(self, command: list[str]) -> subprocess.CompletedProcess[str]:
        tail = tuple(command[1:])
        if tail == ("plugin", "marketplace", "list", "--json"):
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
                    for name in PLUGIN_NAMES
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
