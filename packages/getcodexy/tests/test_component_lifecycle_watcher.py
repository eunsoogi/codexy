from __future__ import annotations

import os
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_watcher_materialization import (
    materialize_watcher,
    watcher_entrypoint,
)
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleWatcherTests(unittest.TestCase):
    @staticmethod
    def _expected_source(state) -> bytes:
        plugin = state.marketplace / "plugins/codexy"
        relative = (
            "runtime/codexy-mcp-watcher-windows-x86_64.exe"
            if os.name == "nt"
            else "mcp/codexy-mcp-watcher.sh"
        )
        return (plugin / relative).read_bytes()

    def test_install_materializes_the_registered_watcher_entrypoint(self) -> None:
        with fixture() as state:
            seen_mutations: list[tuple[str, ...]] = []

            def materialize_before_host_add(plugin, home):
                seen_mutations.append(tuple(state.mutations))
                return materialize_watcher(plugin, home)

            with patch(
                "codexy_runtime_tools.component_lifecycle_recovery.materialize_watcher",
                side_effect=materialize_before_host_add,
            ):
                receipt = run_operation(
                    "install",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-install-watcher",
                )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(seen_mutations, [()])
            watcher = watcher_entrypoint(state.marketplace / "plugins/codexy")
            self.assertEqual(watcher.read_bytes(), self._expected_source(state))

    def test_update_refreshes_a_stale_materialized_watcher_entrypoint(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            watcher = watcher_entrypoint(state.marketplace / "plugins/codexy")
            watcher.write_bytes(b"stale target")
            seen_mutations: list[tuple[str, ...]] = []

            def materialize_before_host_update(plugin, home):
                seen_mutations.extend(
                    mutation
                    for mutation in state.mutations
                    if mutation[:2] == ("plugin", "add")
                )
                return materialize_watcher(plugin, home)

            with patch(
                "codexy_runtime_tools.component_lifecycle_recovery.materialize_watcher",
                side_effect=materialize_before_host_update,
            ):
                receipt = run_operation(
                    "update",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-update-watcher",
                )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(seen_mutations, [])
            self.assertEqual(watcher.read_bytes(), self._expected_source(state))

    @unittest.skipIf(os.name == "nt", "creating a symlink requires Windows privileges")
    def test_install_rejects_a_symlinked_watcher_parent_before_materialization(
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
                operation_id="op-install-symlinked-watcher-parent",
            )

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(marker.read_bytes(), b"preserve")


if __name__ == "__main__":
    unittest.main()
