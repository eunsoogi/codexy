from __future__ import annotations

import json
import unittest

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import InventorySnapshot, Journal, read_journal, write_journal
from packages.getcodexy.tests.component_lifecycle_support import fixture


class UpdateRecoveryTests(unittest.TestCase):
    def test_started_update_is_resumed_not_inferred_from_unchanged_selection(self) -> None:
        with fixture({"core", "github"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core", "github"]}), encoding="utf-8")
            write_journal(state.home, Journal("op-interrupted-update", "update", ("github",), ("core", "github"), ("core", "github"), ("core", "github"), InventorySnapshot.capture(state.home), "started"))

            receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-after-update")

            upgrade = ("plugin", "marketplace", "upgrade", "codexy", "--json")
            self.assertIn(upgrade, state.calls)
            prior = target.parent / "receipts" / "op-interrupted-update.json"
            self.assertEqual(json.loads(prior.read_text(encoding="utf-8"))["outcome"], "completed")
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])

    def test_interrupted_older_update_recovers_its_canonical_mixed_version_state(self) -> None:
        with fixture({"core", "github"}, versions={"core": "1.2.0", "github": "1.2.0"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core", "github"]}), encoding="utf-8")
            write_journal(state.home, Journal("op-interrupted-older-update", "update", ("github",), ("core", "github"), ("core", "github"), ("core", "github"), InventorySnapshot.capture(state.home), "started"))
            state.versions["core"] = "1.3.0"

            receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-after-older-update")

            self.assertIn(("plugin", "marketplace", "upgrade", "codexy", "--json"), state.calls)
            self.assertIsNone(read_journal(state.home))
            self.assertEqual(json.loads((target.parent / "receipts" / "op-interrupted-older-update.json").read_text(encoding="utf-8"))["outcome"], "completed")
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])


if __name__ == "__main__":
    unittest.main()
