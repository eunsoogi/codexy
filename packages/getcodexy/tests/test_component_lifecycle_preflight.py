from __future__ import annotations

import json
import unittest

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecyclePreflightTests(unittest.TestCase):
    def test_absent_marketplace_does_not_bootstrap_for_a_rejected_request(self) -> None:
        with fixture(marketplace_present=False) as state:
            receipt = run_operation("install", ("unknown",), state.home, state.codex, state.run, operation_id="op-unknown-no-market")
            self.assertEqual(receipt["outcome"], "rejected")
            self.assertFalse(state.marketplace_present)
            self.assertEqual(state.mutations, [])

    def test_absent_marketplace_does_not_bootstrap_for_a_corrupt_journal(self) -> None:
        with fixture(marketplace_present=False) as state:
            target = inventory_path(state.home).parent / "inflight.json"
            target.parent.mkdir(parents=True)
            target.write_text('{"schema":"getcodexy.component-transaction.v1"}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "journal"):
                run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-bad-no-market")
            self.assertFalse(state.marketplace_present)
            self.assertEqual(state.mutations, [])

    def test_older_lockstep_update_failure_preserves_the_prior_version_and_receipt(self) -> None:
        with fixture({"core"}, fail_upgrade=True, versions={"core": "1.2.0"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core"]}), encoding="utf-8")
            receipt = run_operation("update", (), state.home, state.codex, state.run, operation_id="op-old-upgrade-fail")
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core"})
            self.assertEqual(state.versions, {"core": "1.2.0"})
            self.assertEqual(json.loads(target.read_text())["components"], ["core"])
            saved = target.parent / "receipts" / "op-old-upgrade-fail.json"
            self.assertEqual(json.loads(saved.read_text())["outcome"], "rolled-back")


if __name__ == "__main__":
    unittest.main()
