from __future__ import annotations

import unittest
import tempfile
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.monolith_migration import migrate
from codexy_runtime_tools.monolith_migration_plan import MigrationPlan
from codexy_runtime_tools.monolith_migration_state import (
    MigrationJournal,
    journal_path,
    write_journal,
)
from codexy_runtime_tools.component_lifecycle import PreAdmissionError


class MonolithMigrationTests(unittest.TestCase):
    def test_rejected_plan_performs_no_host_mutation(self) -> None:
        rejected = MigrationPlan(
            "rejected",
            "1.3.0",
            "1.3.0",
            (),
            "target-release-unavailable",
            "a distinct release is required",
        )
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration._already_migrated",
                return_value=None,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._discover",
                return_value=(Path("/legacy"), "1.3.0"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration.plan_migration",
                return_value=rejected,
            ),
            patch("codexy_runtime_tools.monolith_migration._activate") as activate,
        ):
            receipt = migrate(Path(directory) / "home", Path("/codex"), lambda _: None)

        self.assertEqual(receipt["outcome"], "rejected")
        self.assertEqual(receipt["errors"], [{"code": "target-release-unavailable"}])
        activate.assert_not_called()

    def test_ready_plan_activates_then_reports_idempotent_selection(self) -> None:
        ready = MigrationPlan(
            "ready", "1.3.0", "1.4.0", ("core", "github", "devtools"), None, ""
        )
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration._already_migrated",
                return_value=None,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._discover",
                return_value=(Path("/legacy"), "1.3.0"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration.plan_migration",
                return_value=ready,
            ),
            patch("codexy_runtime_tools.monolith_migration._stage_target"),
            patch("codexy_runtime_tools.monolith_migration._activate") as activate,
        ):
            receipt = migrate(Path(directory) / "home", Path("/codex"), lambda _: None)

        self.assertEqual(receipt["outcome"], "completed")
        self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
        activate.assert_called_once()

    def test_matching_split_re_run_is_completed_without_legacy_discovery(self) -> None:
        receipt = {
            "schema": "getcodexy.monolith-migration-receipt.v1",
            "outcome": "completed",
            "selection_after": ["core", "devtools"],
        }
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration._already_migrated",
                return_value=receipt,
            ),
            patch("codexy_runtime_tools.monolith_migration._discover") as discover,
        ):
            result = migrate(
                Path(directory) / "home", Path("/codex"), lambda _: None, ("devtools",)
            )

        self.assertEqual(result, receipt)
        discover.assert_not_called()

    def test_ambiguous_host_inventory_is_a_closed_rejection(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration._already_migrated",
                return_value=None,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._discover",
                side_effect=RuntimeError("ambiguous"),
            ),
        ):
            receipt = migrate(Path(directory) / "home", Path("/codex"), lambda _: None)

        self.assertEqual(receipt["outcome"], "rejected")
        self.assertEqual(receipt["errors"], [{"code": "ambiguous-monolith"}])

    def test_host_and_tree_source_versions_must_match_before_staging(self) -> None:
        ready = MigrationPlan("ready", "1.3.0", "1.4.0", ("core",), None, "")
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration._already_migrated",
                return_value=None,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._discover",
                return_value=(Path("/legacy"), "9.9.9"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration.plan_migration",
                return_value=ready,
            ),
            patch("codexy_runtime_tools.monolith_migration._stage_target") as stage,
        ):
            receipt = migrate(Path(directory) / "home", Path("/codex"), lambda _: None)

        self.assertEqual(receipt["errors"], [{"code": "ambiguous-monolith"}])
        stage.assert_not_called()

    def test_target_staging_failure_is_a_rejected_receipt_before_host_mutation(
        self,
    ) -> None:
        ready = MigrationPlan("ready", "1.3.0", "1.4.0", ("core",), None, "")
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration._already_migrated",
                return_value=None,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._discover",
                return_value=(Path("/legacy"), "1.3.0"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration.plan_migration",
                return_value=ready,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._stage_target",
                side_effect=RuntimeError("target unavailable"),
            ),
            patch("codexy_runtime_tools.monolith_migration.write_journal") as write,
            patch("codexy_runtime_tools.monolith_migration._activate") as activate,
        ):
            receipt = migrate(Path(directory) / "home", Path("/codex"), lambda _: None)

        self.assertEqual(receipt["outcome"], "rejected")
        self.assertEqual(receipt["errors"], [{"code": "target-release-unavailable"}])
        write.assert_not_called()
        activate.assert_not_called()

    def test_activation_marks_the_durable_journal_before_host_mutation(self) -> None:
        ready = MigrationPlan("ready", "1.3.0", "1.4.0", ("core",), None, "")
        events: list[str] = []
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration._already_migrated",
                return_value=None,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._discover",
                return_value=(Path("/legacy"), "1.3.0"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration.plan_migration",
                return_value=ready,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration._stage_target",
                side_effect=lambda *_: events.append("stage"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration.write_journal",
                side_effect=lambda _, journal: events.append(journal.phase),
            ),
            patch("codexy_runtime_tools.monolith_migration.clear_journal"),
            patch("codexy_runtime_tools.monolith_migration._activate") as activate,
        ):
            receipt = migrate(Path(directory) / "home", Path("/codex"), lambda _: None)

        self.assertEqual(receipt["outcome"], "completed")
        self.assertEqual(events, ["stage", "prepared", "activating"])
        activate.assert_called_once()

    def test_active_component_transaction_rejects_migration_before_host_work(
        self,
    ) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch(
                "codexy_runtime_tools.monolith_migration.transaction_lock",
                side_effect=PreAdmissionError("active"),
            ),
            patch("codexy_runtime_tools.monolith_migration._discover") as discover,
        ):
            receipt = migrate(Path(directory) / "home", Path("/codex"), lambda _: None)

        self.assertEqual(receipt["errors"], [{"code": "migration-in-progress"}])
        discover.assert_not_called()

    def test_pending_journal_rolls_back_before_a_new_migration_is_admitted(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            write_journal(
                home,
                MigrationJournal.capture(home, "1.3.0", "1.4.0", ("core",)),
            )
            with patch("codexy_runtime_tools.monolith_migration._rollback") as rollback:
                receipt = migrate(home, Path("/codex"), lambda _: None)

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertFalse(journal_path(home).exists())
            rollback.assert_called_once()


if __name__ == "__main__":
    unittest.main()
