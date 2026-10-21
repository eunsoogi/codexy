from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor, status
from codexy_runtime_tools.plugin_resolution import marketplace_identity
from packages.getcodexy.tests.local_marketplace_identity_fixture import (
    LocalHost,
)


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

        self.assertEqual((identity.source_type, identity.root), ("local", root))
        expected = {"sourceType": "local", "source": str(root)}
        self.assertEqual(identity.host_source, expected)

    def test_local_archive_install_and_status_use_local_host_identity(self) -> None:
        with LocalHost() as host:
            receipt = host.operate("install", "op-local-archive")
            observed = status(host.home, codex=host.codex, runner=host.run)
            diagnosed = doctor(host.home, codex=host.codex, runner=host.run)

        expected = ["core", "github", "devtools"]
        self.assertEqual(receipt["outcome"], "completed")
        self.assertEqual(receipt["selection_after"], expected)
        self.assertEqual(observed["installed_components"], expected)
        self.assertEqual(observed["errors"], [])
        self.assertEqual(diagnosed["inventory_consistency"], "consistent")
        self.assertNotIn({"code": "invalid-installed-inventory"}, diagnosed["errors"])

    def test_update_and_bootstrap_preserve_frozen_local_identity(self) -> None:
        for command in ("update", "bootstrap"):
            with self.subTest(command=command), LocalHost() as host:
                if command == "update":
                    installed = host.operate("install", "op-local-install")
                    self.assertEqual(installed["outcome"], "completed")
                start, reads = len(host.mutations), host.marketplace_reads
                receipt = host.operate(command, f"op-local-{command}")
                mutations = host.mutations[start:]

                self.assertEqual(receipt["outcome"], "completed")
                self.assertEqual(host.marketplace_reads, reads + 2)
                self.assertEqual(
                    receipt["selection_after"], ["core", "github", "devtools"]
                )
                self.assertNotIn(
                    ("plugin", "marketplace", "upgrade", "codexy", "--json"),
                    mutations,
                )

    def test_foreign_unbound_or_ambiguous_local_sources_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
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
            host.installed_source = {  # Simulate a source outside the selected root.
                "sourceType": "local",
                "source": str(foreign),
            }
            receipt = host.operate("install", "op-mismatched-local-source", ("github",))

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
                receipt = host.operate("install", f"op-provenance-{field}")

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(host.mutations, [])
