from __future__ import annotations

import base64
import json
import unittest

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_transaction_state import inventory_path
from packages.getcodexy.tests.component_lifecycle_support import fixture


class JournalValidationTests(unittest.TestCase):
    def test_duplicate_key_inventory_snapshot_is_rejected_before_host_mutation(self) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home).parent / "inflight.json"
            target.parent.mkdir(parents=True)
            snapshot = base64.b64encode(b'{"schema":"bad","schema":"getcodexy.installed-component-inventory.v1","components":["core"]}').decode()
            target.write_text(json.dumps({"schema": "getcodexy.component-transaction.v1", "operation_id": "op-duplicate", "command": "install", "requested": ["github"], "resolved": ["core", "github"], "before": ["core"], "target": ["core", "github"], "inventory": snapshot, "phase": "started"}), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "duplicate keys"):
                run_operation("install", ("github",), state.home, state.codex, state.run, operation_id="op-next")
            self.assertEqual(state.mutations, [])


if __name__ == "__main__":
    unittest.main()
