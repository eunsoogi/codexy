from __future__ import annotations

import os
import shutil
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
            plugin = Path(temporary).resolve()
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
            plugin = Path(temporary).resolve()
            with (
                patch.object(materializer.os, "name", "posix"),
                self.assertRaisesRegex(RuntimeError, "source launcher is missing"),
            ):
                materializer.materialize_watcher(plugin)
            self.assertFalse((plugin / materializer.WATCHER_COMMAND).exists())

    def test_posix_rejects_a_symlinked_target_parent_before_copy(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            plugin = root / "plugin"
            outside = root / "outside/mcp"
            outside.mkdir(parents=True)
            source = outside / "codexy-mcp-watcher.sh"
            source.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            source.chmod(source.stat().st_mode | stat.S_IXUSR)
            marker = outside / "codexy-mcp-watcher"
            marker.write_bytes(b"preserve")
            plugin.mkdir()
            (plugin / "mcp").symlink_to(outside, target_is_directory=True)

            with (
                patch.object(materializer.os, "name", "posix"),
                self.assertRaisesRegex(ValueError, "symlink"),
            ):
                materializer.materialize_watcher(plugin)
            self.assertEqual(marker.read_bytes(), b"preserve")

    def test_existing_host_cache_repairs_the_exact_versioned_target(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source_plugin = root / "marketplace/plugins/codexy"
            source = source_plugin / materializer.WATCHER_SOURCE
            source.parent.mkdir(parents=True)
            source.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            source.chmod(source.stat().st_mode | stat.S_IXUSR)
            home = root / "home/.codex"
            cache_plugin = home / materializer.WATCHER_CACHE_ROOT / "1.2.2"
            shutil.copytree(source_plugin, cache_plugin)

            with patch.object(materializer.os, "name", "posix"):
                self.assertFalse(materializer.valid_watcher_cache(home, "1.2.2"))
                target = materializer.materialize_watcher_cache(home, "1.2.2")

            self.assertEqual(target, cache_plugin / materializer.WATCHER_COMMAND)
            self.assertEqual(target.read_bytes(), source.read_bytes())
            self.assertTrue(materializer.valid_watcher_cache(home, "1.2.2"))

    @unittest.skipIf(os.name == "nt", "creating a symlink requires Windows privileges")
    def test_cache_root_symlink_is_not_followed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            home = root / "home/.codex"
            outside = root / "outside"
            outside.mkdir(parents=True)
            home.mkdir(parents=True)
            (home / "plugins").mkdir()
            (home / "plugins/cache").symlink_to(outside, target_is_directory=True)

            self.assertFalse(materializer.valid_watcher_cache(home, "1.2.2"))
            with self.assertRaisesRegex(RuntimeError, "regular directory"):
                materializer.materialize_watcher_cache(home, "1.2.2")

    def test_windows_prefers_the_bundled_native_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = Path(temporary).resolve()
            bundled = plugin / materializer.WATCHER_RUNTIME
            bundled.parent.mkdir(parents=True)
            bundled.write_bytes(b"native watcher")
            bundled.chmod(bundled.stat().st_mode | stat.S_IXUSR)

            with (
                patch.object(materializer.os, "name", "nt"),
                patch("codexy_runtime_tools.updater.Path", PosixPath),
            ):
                target = materializer.materialize_watcher(plugin)

            self.assertEqual(target, plugin / materializer.WATCHER_WINDOWS)
            self.assertEqual(target.read_bytes(), bundled.read_bytes())

    def test_windows_downloads_the_official_runtime_when_source_has_no_binary(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
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
                patch("codexy_runtime_tools.updater.Path", PosixPath),
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
