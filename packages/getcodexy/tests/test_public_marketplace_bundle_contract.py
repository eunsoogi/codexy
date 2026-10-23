"""Exercise the Watcher rules applied after public bundle integrity checks."""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from scripts.public_marketplace_bundle_support import (
    COMPONENTS,
    LEGACY_UPGRADE_VERSION,
    verify_contents,
)
from packages.getcodexy.tests.component_watcher_fixture import install_watcher_runtime
from packages.getcodexy.tests.component_watcher_fixture import (
    verified_legacy_marketplace_root,
)
from packages.getcodexy.tests.component_marketplace_fixture import populate_plugins


class PublicMarketplaceBundleContractTests(unittest.TestCase):
    def test_no_watcher_public_bundle_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            receipt = self._package_contents(root, "2.0.0", include_watcher=False)

            self.assertIsNone(verify_contents(root, receipt, "2.0.0"))

    def test_legacy_upgrade_requires_and_returns_the_native_watcher(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            receipt = self._package_contents(
                root, LEGACY_UPGRADE_VERSION, include_watcher=True
            )

            watcher = verify_contents(root, receipt, LEGACY_UPGRADE_VERSION)

            self.assertEqual(
                watcher,
                root / "plugins/codexy/mcp/codexy-mcp-watcher.exe",
            )

    def test_legacy_upgrade_rejects_a_missing_watcher(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            receipt = self._package_contents(
                root, LEGACY_UPGRADE_VERSION, include_watcher=False
            )

            with self.assertRaisesRegex(SystemExit, "1.13.4 upgrade bundle"):
                verify_contents(root, receipt, LEGACY_UPGRADE_VERSION)

    def test_newer_public_bundle_rejects_a_legacy_watcher(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            receipt = self._package_contents(root, "2.0.0", include_watcher=True)

            with self.assertRaisesRegex(SystemExit, "outside the verified 1.13.4"):
                verify_contents(root, receipt, "2.0.0")

    def test_watcher_fixture_builder_rejects_a_newer_package(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = Path(temporary)
            manifest = plugin / ".codex-plugin/plugin.json"
            manifest.parent.mkdir(parents=True)
            manifest.write_text(
                json.dumps({"name": "codexy", "version": "2.0.0"}),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(RuntimeError, "restricted to the 1.13.4"):
                install_watcher_runtime(plugin, verified_legacy_upgrade_fixture=True)

            self.assertFalse((plugin / "runtime").exists())

    def test_default_checkout_fixture_does_not_inherit_the_verified_watcher(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            marketplace = Path(temporary)
            populate_plugins(marketplace)

            core = marketplace / "plugins/codexy"
            self.assertEqual(
                (core / "hooks/hooks.json").read_bytes(),
                (
                    Path(__file__).resolve().parents[3]
                    / "plugins/codexy/hooks/hooks.json"
                ).read_bytes(),
            )
            self.assertFalse(
                (core / "runtime/codexy-mcp-watcher-windows-x86_64.exe").exists()
            )

    def test_explicit_upgrade_fixture_uses_the_verified_legacy_binary(self) -> None:
        legacy_root = verified_legacy_marketplace_root()
        if legacy_root is None:
            self.skipTest("no verified 1.13.4 public bundle is configured")

        with tempfile.TemporaryDirectory() as temporary:
            marketplace = Path(temporary)
            populate_plugins(marketplace, verified_legacy_upgrade_fixture=True)

            core = marketplace / "plugins/codexy"
            source = legacy_root / "plugins/codexy/mcp/codexy-mcp-watcher.exe"
            self.assertEqual(source.read_bytes()[:2], b"MZ")
            self.assertEqual(
                (core / "hooks/hooks.json").read_bytes(),
                (
                    Path(__file__).resolve().parents[3]
                    / "plugins/codexy/hooks/hooks.json"
                ).read_bytes(),
            )
            if os.name == "nt":
                fixture_binary = core / "runtime/codexy-mcp-watcher-windows-x86_64.exe"
                self.assertEqual(fixture_binary.read_bytes(), source.read_bytes())

    @staticmethod
    def _package_contents(
        root: Path, version: str, *, include_watcher: bool
    ) -> dict[str, object]:
        records = []
        marketplace_plugins = []
        for plugin, package_root in COMPONENTS.items():
            manifest_path = root / package_root / ".codex-plugin/plugin.json"
            manifest_path.parent.mkdir(parents=True, exist_ok=True)
            manifest_path.write_text(
                json.dumps({"name": plugin, "version": version}), encoding="utf-8"
            )
            records.append(
                {
                    "plugin": plugin,
                    "packageRoot": package_root,
                    "version": version,
                    "manifestSha256": hashlib.sha256(
                        manifest_path.read_bytes()
                    ).hexdigest(),
                }
            )
            marketplace_plugins.append(
                {
                    "name": plugin,
                    "version": version,
                    "source": {"path": f"./{package_root}"},
                }
            )
        marketplace_path = root / ".agents/plugins/marketplace.json"
        marketplace_path.parent.mkdir(parents=True, exist_ok=True)
        marketplace_path.write_text(
            json.dumps({"plugins": marketplace_plugins}), encoding="utf-8"
        )
        if include_watcher:
            watcher = root / "plugins/codexy/mcp/codexy-mcp-watcher.exe"
            watcher.parent.mkdir(parents=True, exist_ok=True)
            watcher.write_bytes(b"MZfixture")
        return {"components": records}


if __name__ == "__main__":
    unittest.main()
