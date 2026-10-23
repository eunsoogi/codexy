"""MCP materialization validates source and repairs only the selected cache."""

from __future__ import annotations

import json
import os
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from unittest.mock import patch

from codexy_runtime_tools import component_mcp_cache, mcp_bootstrap
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_mcp_materialization import (
    component_cache_plugin,
    materialize_component_mcp,
    materialize_component_mcp_cache,
    mcp_configuration,
    valid_component_mcp,
    valid_component_mcp_cache,
)
from packages.getcodexy.tests.component_marketplace_fixture import (
    copy_plugin_package as _copy_plugin,
)
from packages.getcodexy.tests.component_watcher_fixture import LEGACY_VERSION


REPOSITORY = Path(__file__).resolve().parents[3]
VERSION = load_component_manifest().version


class WatcherMaterializationTests(unittest.TestCase):
    def test_core_materialization_validates_the_source_without_an_alias_target(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = _copy_plugin(Path(temporary), "codexy")

            self.assertEqual(materialize_component_mcp(plugin, "core", VERSION), plugin)
            self.assertTrue(valid_component_mcp(plugin, "core", VERSION))
            self.assertFalse((plugin / ".mcp.json").exists())
            self.assertFalse((plugin / "mcp/codexy-mcp-watcher").exists())

    def test_each_component_uses_only_its_own_mcp_servers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            # Core 2.0 is no-MCP; retain Watcher registration coverage on its legacy package.
            for component, plugin_name, servers, version in (
                ("core", "codexy", {"watcher"}, LEGACY_VERSION),
                ("devtools", "codexy-devtools", {"lsp", "codegraph"}, VERSION),
            ):
                plugin = _copy_plugin(root, plugin_name, version)
                configuration = json.loads(
                    (plugin / ".mcp.json").read_text(encoding="utf-8")
                )
                self.assertEqual(set(configuration), servers)
                self.assertEqual(configuration, mcp_configuration(component, version))
                self.assertTrue(valid_component_mcp(plugin, component, version))

    def test_devtools_helper_is_readable_but_not_required_to_be_executable(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = _copy_plugin(Path(temporary), "codexy-devtools")
            helper = plugin / "mcp/runtime-platform.sh"
            helper.chmod(0o644)

            materialize_component_mcp(plugin, "devtools", VERSION)

            self.assertTrue(valid_component_mcp(plugin, "devtools", VERSION))
            self.assertEqual(stat.S_IMODE(helper.stat().st_mode), 0o644)

    def test_source_rejects_a_release_version_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = _copy_plugin(Path(temporary), "codexy")

            with self.assertRaisesRegex(RuntimeError, "does not match"):
                materialize_component_mcp(plugin, "core", "9.9.9")

    def test_source_rejects_a_nonmatching_plugin_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = _copy_plugin(Path(temporary), "codexy")
            manifest_path = plugin / ".codex-plugin/plugin.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["name"] = "codexy-devtools"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "identity is invalid"):
                materialize_component_mcp(plugin, "core", VERSION)

    def test_existing_host_cache_repairs_only_the_exact_component_surface(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            # Cache repair must follow the legacy core MCP surface under test.
            source = _copy_plugin(root / "marketplace", "codexy", LEGACY_VERSION)
            home = root / "home/.codex"
            cache = component_cache_plugin(home, "core", LEGACY_VERSION)
            cache.parent.mkdir(parents=True)
            shutil.copytree(source, cache)
            bootstrap = cache / "mcp/codexy_mcp_bootstrap.py"
            bootstrap.write_text("stale\n", encoding="utf-8")
            bootstrap.chmod(0o600)

            target = materialize_component_mcp_cache(
                home, "core", LEGACY_VERSION, source_plugin=source
            )

            self.assertEqual(target, cache)
            self.assertEqual(
                bootstrap.read_bytes(),
                (source / "mcp/codexy_mcp_bootstrap.py").read_bytes(),
            )
            self.assertEqual(
                stat.S_IMODE(bootstrap.stat().st_mode),
                stat.S_IMODE((source / "mcp/codexy_mcp_bootstrap.py").stat().st_mode),
            )
            self.assertTrue(valid_component_mcp_cache(home, "core", LEGACY_VERSION))
            self.assertFalse((cache / "mcp/codexy-mcp-watcher").exists())

    def test_missing_selected_host_cache_is_not_silently_skipped(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = _copy_plugin(root / "marketplace", "codexy")
            home = root / "home/.codex"
            (home / "plugins/cache").mkdir(parents=True)

            with self.assertRaisesRegex(RuntimeError, "MCP cache plugin is missing"):
                materialize_component_mcp_cache(
                    home, "core", VERSION, source_plugin=source
                )

            self.assertFalse(component_cache_plugin(home, "core", VERSION).exists())

    def test_same_version_cache_reuses_an_unchanged_surface(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = _copy_plugin(root / "marketplace", "codexy")
            home = root / "home/.codex"
            cache = component_cache_plugin(home, "core", VERSION)
            cache.parent.mkdir(parents=True)
            shutil.copytree(source, cache)

            with patch.object(component_mcp_cache, "_atomic_copy") as copy_file:
                materialize_component_mcp_cache(
                    home, "core", VERSION, source_plugin=source
                )

            copy_file.assert_not_called()

    @unittest.skipIf(os.name == "nt", "creating a symlink requires Windows privileges")
    def test_cache_root_symlink_is_not_followed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / "home/.codex"
            outside = root / "outside"
            outside.mkdir(parents=True)
            home.mkdir(parents=True)
            (home / "plugins").mkdir()
            (home / "plugins/cache").symlink_to(outside, target_is_directory=True)

            self.assertFalse(valid_component_mcp_cache(home, "core", VERSION))
            with self.assertRaisesRegex(ValueError, "symlink"):
                materialize_component_mcp_cache(home, "core", VERSION)

    def test_bootstrap_reads_the_selected_manifest_version(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = _copy_plugin(Path(temporary), "codexy-devtools")
            with (
                patch.object(mcp_bootstrap.Path, "cwd", return_value=plugin),
                patch.object(
                    mcp_bootstrap.shutil, "which", return_value="/usr/bin/uvx"
                ),
                patch.object(mcp_bootstrap.os, "name", "posix"),
                patch.object(
                    mcp_bootstrap.os,
                    "execvpe",
                    side_effect=OSError("test stop"),
                ) as execvpe,
            ):
                result = mcp_bootstrap.main(["codegraph", "--stdio"])

            self.assertEqual(result, 127)
            executable, command, environment = execvpe.call_args.args
            self.assertEqual(executable, "/usr/bin/uvx")
            self.assertEqual(
                command,
                [
                    "/usr/bin/uvx",
                    "--from",
                    f"getcodexy=={VERSION}",
                    "codexy-mcp-runtime",
                    "codegraph",
                    "--plugin-root",
                    str(plugin),
                    "--",
                    "--stdio",
                ],
            )
            self.assertEqual(environment["CODEXY_PLUGIN_ROOT"], str(plugin))

    def test_bootstrap_uses_a_child_process_on_windows(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            plugin = _copy_plugin(Path(temporary), "codexy-devtools")
            with (
                patch.object(mcp_bootstrap.Path, "cwd", return_value=plugin),
                patch.object(
                    mcp_bootstrap.shutil, "which", return_value="C:/uv/uvx.exe"
                ),
                patch.object(mcp_bootstrap.os, "name", "nt"),
                patch.object(
                    mcp_bootstrap.subprocess,
                    "run",
                    return_value=subprocess.CompletedProcess([], 23),
                ) as run,
            ):
                result = mcp_bootstrap.main(["codegraph", "--stdio"])

            self.assertEqual(result, 23)
            command = run.call_args.args[0]
            self.assertEqual(command[0], "C:/uv/uvx.exe")
            self.assertEqual(command[2], f"getcodexy=={VERSION}")
            self.assertEqual(command[-1], "--stdio")
            self.assertEqual(
                run.call_args.kwargs["env"]["CODEXY_PLUGIN_ROOT"], str(plugin)
            )
            self.assertFalse(run.call_args.kwargs.get("shell", False))

    def test_only_devtools_packages_the_shared_bootstrap(self) -> None:
        canonical = (
            REPOSITORY / "packages/getcodexy/src/codexy_runtime_tools/mcp_bootstrap.py"
        ).read_bytes()
        self.assertFalse(
            (REPOSITORY / "plugins/codexy/mcp/codexy_mcp_bootstrap.py").exists()
        )
        self.assertEqual(
            (
                REPOSITORY
                / "plugins/codexy-devtools/mcp/codexy_mcp_bootstrap.py"
            ).read_bytes(),
            canonical,
        )


if __name__ == "__main__":
    unittest.main()
