from __future__ import annotations

import os
import shutil
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_mcp_materialization import (
    materialize_component_mcp,
)
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import VERSION, fixture


class LifecycleWatcherTests(unittest.TestCase):
    def test_install_validates_each_selected_mcp_source_before_host_add(self) -> None:
        with fixture() as state:
            calls: list[tuple[str, str, tuple[tuple[str, ...], ...]]] = []

            def validate_source(plugin, component, version):
                calls.append((component, version, tuple(state.mutations)))
                return materialize_component_mcp(plugin, component, version)

            with patch(
                "codexy_runtime_tools.component_lifecycle_mcp.materialize_component_mcp",
                side_effect=validate_source,
            ):
                receipt = run_operation(
                    "install",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-install-mcp",
                )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(
                [(component, version) for component, version, _ in calls],
                [("core", VERSION), ("devtools", VERSION)],
            )
            self.assertTrue(
                all(
                    not any(mutation[:2] == ("plugin", "add") for mutation in snapshot)
                    for _, _, snapshot in calls
                )
            )
            core = state.marketplace / "plugins/codexy"
            self.assertTrue((core / "mcp/codexy_mcp_bootstrap.py").is_file())
            self.assertFalse((core / "mcp/codexy-mcp-watcher").exists())

    def test_update_validates_the_core_source_before_host_update(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            calls: list[tuple[str, str, tuple[tuple[str, ...], ...]]] = []

            def validate_source(plugin, component, version):
                calls.append((component, version, tuple(state.mutations)))
                return materialize_component_mcp(plugin, component, version)

            with patch(
                "codexy_runtime_tools.component_lifecycle_mcp.materialize_component_mcp",
                side_effect=validate_source,
            ):
                receipt = run_operation(
                    "update",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-update-mcp",
                )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(
                [(component, version) for component, version, _ in calls],
                [("core", VERSION)],
            )
            self.assertTrue(
                all(
                    not any(mutation[:2] == ("plugin", "add") for mutation in snapshot)
                    for _, _, snapshot in calls
                )
            )

    def test_update_repairs_a_stale_host_cache_surface(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            source_plugin = state.marketplace / "plugins/codexy"
            materialize_component_mcp(source_plugin, "core", VERSION)
            cache = state.home / "plugins/cache/codexy/codexy" / VERSION
            shutil.copytree(source_plugin, cache)
            cache_bootstrap = cache / "mcp/codexy_mcp_bootstrap.py"
            cache_bootstrap.write_text("stale cache surface\n", encoding="utf-8")

            receipt = run_operation(
                "update",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update-cache-mcp",
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(
                cache_bootstrap.read_bytes(),
                (source_plugin / "mcp/codexy_mcp_bootstrap.py").read_bytes(),
            )

    def test_install_rolls_back_when_selected_host_cache_is_missing(self) -> None:
        with fixture() as state:
            (state.home / "plugins/cache").mkdir(parents=True)

            receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-install-missing-mcp-cache",
            )

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertNotIn("core", state.selection)
            self.assertFalse(
                (state.home / "plugins/cache/codexy/codexy" / VERSION).exists()
            )

    @unittest.skipIf(os.name == "nt", "creating a symlink requires Windows privileges")
    def test_install_rejects_a_symlinked_mcp_parent_before_materialization(
        self,
    ) -> None:
        with fixture() as state:
            core = state.marketplace / "plugins/codexy"
            outside = state.root / "outside"
            outside.mkdir()
            moved = outside / "mcp"
            (core / "mcp").rename(moved)
            marker = moved / "codexy-mcp-watcher"
            marker.write_bytes(b"preserve")
            (core / "mcp").symlink_to(moved, target_is_directory=True)

            receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-install-symlinked-mcp-parent",
            )

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(marker.read_bytes(), b"preserve")


if __name__ == "__main__":
    unittest.main()
