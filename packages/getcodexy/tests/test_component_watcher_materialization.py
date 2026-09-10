from __future__ import annotations

import os
import stat
import tempfile
import unittest
from pathlib import Path, PosixPath
from types import SimpleNamespace
from unittest.mock import patch

from codexy_runtime_tools import component_watcher_materialization as materializer


class WatcherMaterializationTests(unittest.TestCase):
    def test_posix_materializes_the_registered_entrypoint_from_the_source_launcher(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = Path(temporary)
            source = plugin / materializer.WATCHER_SOURCE
            source.parent.mkdir(parents=True)
            source.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            source.chmod(source.stat().st_mode | stat.S_IXUSR)

            with patch.object(materializer.os, "name", "posix"):
                target = materializer.materialize_watcher(plugin)

            self.assertEqual(target, plugin / materializer.WATCHER_COMMAND)
            self.assertEqual(target.read_bytes(), source.read_bytes())
            self.assertTrue(target.stat().st_mode & stat.S_IXUSR)
            self.assertTrue(materializer.valid_watcher_entrypoint(plugin))

    def test_posix_missing_source_does_not_create_a_registered_target(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = Path(temporary)
            with (
                patch.object(materializer.os, "name", "posix"),
                self.assertRaisesRegex(RuntimeError, "source launcher is missing"),
            ):
                materializer.materialize_watcher(plugin)
            self.assertFalse((plugin / materializer.WATCHER_COMMAND).exists())

    def test_windows_prefers_the_bundled_native_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = Path(temporary)
            bundled = plugin / materializer.WATCHER_RUNTIME
            bundled.parent.mkdir(parents=True)
            bundled.write_bytes(b"native watcher")
            bundled.chmod(bundled.stat().st_mode | stat.S_IXUSR)

            with patch.object(materializer.os, "name", "nt"):
                target = materializer.materialize_watcher(plugin)

            self.assertEqual(target, plugin / materializer.WATCHER_WINDOWS)
            self.assertEqual(target.read_bytes(), bundled.read_bytes())

    def test_windows_downloads_the_official_runtime_when_source_has_no_binary(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            plugin = root / "plugins/codexy"
            plugin.mkdir(parents=True)
            home = root / "home/.codex"
            config = SimpleNamespace(
                runtime_name="codexy-mcp-watcher-windows-x86_64.exe"
            )

            def install(_config, _work, staged):
                staged.write_bytes(b"downloaded watcher")
                staged.chmod(staged.stat().st_mode | stat.S_IXUSR)

            with (
                patch.object(materializer.os, "name", "nt"),
                patch.object(materializer, "Path", PosixPath),
                patch.dict(os.environ, {}, clear=True),
                patch.object(
                    materializer.Configuration, "load", return_value=config
                ) as load,
                patch.object(
                    materializer, "install_package", side_effect=install
                ) as install_package,
            ):
                target = materializer.materialize_watcher(plugin, home)

            self.assertEqual(target.read_bytes(), b"downloaded watcher")
            load.assert_called_once_with("watcher", plugin, ["--stdio"])
            self.assertEqual(install_package.call_count, 1)


if __name__ == "__main__":
    unittest.main()
