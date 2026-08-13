from __future__ import annotations

import json
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import read_journal
from packages.getcodexy.tests.component_lifecycle_support import fixture


OFFICIAL = "https://github.com/eunsoogi/codexy.git"


class ComponentLifecycleTests(unittest.TestCase):
    def test_bare_install_records_and_reads_back_all_components(self) -> None:
        with fixture() as state:
            receipt = run_operation("install", (), state.home, state.codex, state.run, operation_id="op-install")

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["selection_before"], [])
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertEqual(receipt["installed_components"], ["core", "github", "devtools"])
            self.assertEqual(state.selection, {"core", "github", "devtools"})
            self.assertEqual(recorded(state.home), ["core", "github", "devtools"])

    def test_bare_install_bootstraps_the_official_marketplace(self) -> None:
        with fixture(marketplace_present=False) as state:
            receipt = run_operation("install", (), state.home, state.codex, state.run, operation_id="op-bootstrap-market")
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertTrue(state.marketplace_present)
            self.assertIn(("plugin", "marketplace", "add", "eunsoogi/codexy", "--ref", "main", "--json"), state.mutations)

    def test_explicit_update_preserves_the_selection(self) -> None:
        with fixture({"core", "github", "devtools"}) as state:
            record(state.home, ["core", "github", "devtools"])
            receipt = run_operation("update", ("github",), state.home, state.codex, state.run, operation_id="op-update")

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["resolved_components"], ["core", "github"])
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertEqual(state.selection, {"core", "github", "devtools"})
            self.assertIn(("plugin", "marketplace", "upgrade", "codexy", "--json"), state.calls)
            self.assertIn(("plugin", "add", "codexy@codexy", "--json"), state.calls)

    def test_selective_install_closes_dependencies_and_keeps_existing_components(self) -> None:
        with fixture({"core", "devtools"}) as state:
            record(state.home, ["core", "devtools"])
            receipt = run_operation("install", ("github",), state.home, state.codex, state.run, operation_id="op-selective")

            self.assertEqual(receipt["resolved_components"], ["core", "github"])
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertEqual(state.selection, {"core", "github", "devtools"})

    def test_remove_rejects_a_dependency_protected_component_before_mutation(self) -> None:
        with fixture({"core", "github"}) as state:
            record(state.home, ["core", "github"])
            receipt = run_operation("remove", ("core",), state.home, state.codex, state.run, operation_id="op-guard")

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(receipt["errors"], [{"code": "dependency-protected-removal"}])
            self.assertEqual(state.selection, {"core", "github"})
            self.assertEqual(state.mutations, [])

    def test_failed_mutation_restores_selection_and_record(self) -> None:
        with fixture({"core", "github"}, fail_add="codexy-devtools") as state:
            record(state.home, ["core", "github"])
            receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-failure")

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(receipt["selection_after"], ["core", "github"])
            self.assertEqual(state.selection, {"core", "github"})
            self.assertEqual(recorded(state.home), ["core", "github"])
            stored = inventory_path(state.home).parent / "receipts" / "op-failure.json"
            self.assertEqual(json.loads(stored.read_text(encoding="utf-8"))["outcome"], "rolled-back")

    def test_failed_rollback_remains_a_rollback_on_next_recovery(self) -> None:
        with fixture({"core"}, fail_add="codexy-github", fail_remove="codexy-github") as state:
            record(state.home, ["core"])
            with self.assertRaisesRegex(RuntimeError, "durable recovery"):
                run_operation("install", ("github",), state.home, state.codex, state.run, operation_id="op-retry-rollback")
            self.assertEqual(read_journal(state.home).phase, "rolling-back")
            self.assertEqual(state.selection, {"core", "github"})
            state.fail_remove = None
            receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-after-retry")
            self.assertEqual(json.loads((inventory_path(state.home).parent / "receipts" / "op-retry-rollback.json").read_text())["outcome"], "rolled-back")
            self.assertEqual(receipt["selection_after"], ["core", "devtools"])

    def test_update_failure_restores_its_exact_selection(self) -> None:
        with fixture({"core", "github"}, fail_add="codexy-github") as state:
            record(state.home, ["core", "github"])
            receipt = run_operation("update", ("github",), state.home, state.codex, state.run, operation_id="op-update-fail")
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core", "github"})

    def test_partial_remove_failure_restores_its_exact_selection(self) -> None:
        with fixture({"core", "github", "devtools"}, fail_remove="codexy-github") as state:
            record(state.home, ["core", "github", "devtools"])
            receipt = run_operation("remove", ("github", "devtools"), state.home, state.codex, state.run, operation_id="op-remove-fail")
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core", "github", "devtools"})

    def test_multi_remove_allows_removing_core_with_its_dependent(self) -> None:
        with fixture({"core", "github"}) as state:
            record(state.home, ["core", "github"])
            receipt = run_operation("remove", ("core", "github"), state.home, state.codex, state.run, operation_id="op-remove-both")

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["selection_after"], [])
            self.assertEqual(recorded(state.home), [])
            removals = [call[2] for call in state.mutations if call[:2] == ("plugin", "remove")]
            self.assertEqual(removals, ["codexy-github@codexy", "codexy@codexy"])

    def test_update_without_a_recorded_selection_is_rejected(self) -> None:
        with fixture({"core"}) as state:
            receipt = run_operation("update", (), state.home, state.codex, state.run, operation_id="op-missing-record")

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(receipt["errors"], [{"code": "no-recorded-selection"}])
            self.assertEqual(state.mutations, [])

    def test_remove_requires_a_component_operand(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            receipt = run_operation("remove", (), state.home, state.codex, state.run, operation_id="op-missing-remove")

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(receipt["errors"], [{"code": "missing-removal-target"}])
            self.assertEqual(state.mutations, [])

    def test_interrupted_mutation_restores_selection_and_record(self) -> None:
        with fixture({"core"}, interrupt_add="codexy-github") as state:
            record(state.home, ["core"])
            receipt = run_operation("install", ("github",), state.home, state.codex, state.run, operation_id="op-interrupted")

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core"})
            self.assertEqual(recorded(state.home), ["core"])

    def test_stale_record_is_rejected_without_a_host_mutation(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core", "github"])
            receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-stale")

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(receipt["errors"], [{"code": "inconsistent-installed-state"}])
            self.assertEqual(state.mutations, [])
            saved = inventory_path(state.home).parent / "receipts" / "op-stale.json"
            self.assertEqual(json.loads(saved.read_text(encoding="utf-8"))["selection_before"], ["core"])

    def test_symlinked_transaction_storage_is_rejected_without_a_host_mutation(self) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.symlink_to(state.root / "outside")
            receipt = run_operation("install", ("github",), state.home, state.codex, state.run, operation_id="op-link")

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(state.mutations, [])

    def test_unsafe_operation_id_is_rejected_before_transaction_storage(self) -> None:
        with fixture() as state:
            with self.assertRaisesRegex(ValueError, "safe op-"):
                run_operation("install", (), state.home, state.codex, state.run, operation_id="../escape")
            self.assertEqual(state.mutations, [])

    def test_an_existing_lifecycle_lock_refuses_a_second_operation(self) -> None:
        with fixture() as state:
            from codexy_runtime_tools.component_transaction_state import transaction_lock
            with patch("fcntl.flock", side_effect=BlockingIOError), self.assertRaisesRegex(RuntimeError, "another getcodexy"):
                with transaction_lock(state.home):
                    pass
            self.assertEqual(state.mutations, [])

    def test_next_operation_recovers_a_durable_interrupted_journal(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            # The journal persists before an interrupted host mutation that never completed.
            from codexy_runtime_tools.component_transaction_state import InventorySnapshot, Journal, write_journal
            write_journal(state.home, Journal("op-recover", "install", ("github",), ("core", "github"), ("core",), ("core", "github"), InventorySnapshot.capture(state.home), "started"))
            receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-after-recovery")

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(state.selection, {"core", "devtools"})
            self.assertIsNone(read_journal(state.home))

    def test_recovery_commits_a_journal_when_host_readback_reached_target(self) -> None:
        with fixture({"core", "github"}) as state:
            from codexy_runtime_tools.component_transaction_state import InventorySnapshot, Journal, write_journal
            write_journal(state.home, Journal("op-complete", "install", ("github",), ("core", "github"), (), ("core", "github"), InventorySnapshot.capture(state.home), "started"))
            receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-next")

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(recorded(state.home), ["core", "github", "devtools"])
            prior = inventory_path(state.home).parent / "receipts" / "op-complete.json"
            self.assertEqual(json.loads(prior.read_text(encoding="utf-8"))["outcome"], "completed")

    def test_corrupt_journal_is_rejected_without_a_host_mutation(self) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home).parent / "inflight.json"
            target.parent.mkdir(parents=True)
            target.write_text('{"schema":"getcodexy.component-transaction.v1"}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "journal"):
                run_operation("install", ("github",), state.home, state.codex, state.run, operation_id="op-bad-journal")
            self.assertEqual(state.mutations, [])

    def test_plan_inconsistent_journal_is_rejected_without_a_host_mutation(self) -> None:
        with fixture({"core", "github"}) as state:
            from codexy_runtime_tools.component_transaction_state import InventorySnapshot, Journal, write_journal
            record(state.home, ["core", "github"])
            write_journal(state.home, Journal("op-bad-plan", "install", ("github",), (), ("core", "github"), (), InventorySnapshot.capture(state.home), "started"))
            with self.assertRaisesRegex(ValueError, "journal"):
                run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-after-bad")
            self.assertEqual(state.mutations, [])

    def test_coherent_older_component_version_can_update(self) -> None:
        with fixture({"core"}, versions={"core": "1.2.0"}) as state:
            record(state.home, ["core"])
            receipt = run_operation("update", (), state.home, state.codex, state.run, operation_id="op-old")
            self.assertEqual(receipt["outcome"], "completed")
            self.assertIn(("plugin", "marketplace", "upgrade", "codexy", "--json"), state.calls)


def record(home: Path, components: list[str]) -> None:
    target = inventory_path(home)
    target.parent.mkdir(parents=True)
    target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": components}), encoding="utf-8")


def recorded(home: Path) -> list[str]:
    return json.loads(inventory_path(home).read_text(encoding="utf-8"))["components"]


if __name__ == "__main__":
    unittest.main()
