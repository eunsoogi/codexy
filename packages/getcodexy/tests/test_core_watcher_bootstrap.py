"""Core Watcher source launchers exercise the real bootstrap selection path."""

from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from codexy_runtime_tools.runtime_configuration import (
    PUBLIC_BUNDLE_ASSET,
    Configuration,
)
from codexy_runtime_tools.version_lock import default_package_version


REPOSITORY = "https://github.com/eunsoogi/codexy"
VERSION = default_package_version()


class CoreWatcherBootstrapTests(unittest.TestCase):
    """Keep runtime selection usable for old registered core packages."""

    def test_legacy_core_watcher_selects_the_public_bundle_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = root / ".codex-plugin/plugin.json"
            manifest.parent.mkdir()
            manifest.write_text(
                json.dumps(
                    {"name": "codexy", "repository": REPOSITORY, "version": VERSION}
                ),
                encoding="utf-8",
            )
            with mock.patch.dict(os.environ, {}, clear=True):
                config = Configuration.load("watcher", root, ["--stdio"])
            self.assertTrue(
                config.package_url.endswith(f"/v{VERSION}/{PUBLIC_BUNDLE_ASSET}")
            )
            self.assertTrue(config.allow_mixed_plugin_roots)
            self.assertEqual(config.source_identity.package_plugin_root(), "codexy")


if __name__ == "__main__":
    unittest.main()
