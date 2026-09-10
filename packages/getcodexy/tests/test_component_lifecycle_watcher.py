from __future__ import annotations

import unittest

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_watcher_materialization import watcher_entrypoint
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleWatcherTests(unittest.TestCase):
    def test_install_materializes_the_registered_watcher_entrypoint(self) -> None:
        with fixture() as state:
            receipt = run_operation(
                "install",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-install-watcher",
            )

            self.assertEqual(receipt["outcome"], "completed")
            watcher = watcher_entrypoint(state.marketplace / "plugins/codexy")
            self.assertEqual(
                watcher.read_bytes(),
                (
                    state.marketplace / "plugins/codexy/mcp/codexy-mcp-watcher.sh"
                ).read_bytes(),
            )

    def test_update_refreshes_a_stale_materialized_watcher_entrypoint(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            watcher = watcher_entrypoint(state.marketplace / "plugins/codexy")
            watcher.write_bytes(b"stale target")
            receipt = run_operation(
                "update",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update-watcher",
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(
                watcher.read_bytes(),
                (
                    state.marketplace / "plugins/codexy/mcp/codexy-mcp-watcher.sh"
                ).read_bytes(),
            )


if __name__ == "__main__":
    unittest.main()
