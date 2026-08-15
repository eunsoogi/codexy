from __future__ import annotations

import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

from codexy_runtime_tools.monolith_migration_host import activate, rollback
from codexy_runtime_tools.monolith_migration_plan import MigrationPlan
from codexy_runtime_tools.monolith_migration_state import MigrationJournal


class MonolithMigrationHostTests(unittest.TestCase):
    def test_activation_requires_exact_healthy_component_inventory(self) -> None:
        plan = MigrationPlan("ready", "1.3.0", "1.4.0", ("core", "github"), None, "")
        report = {
            "component_health": [
                {"component": "core", "state": "healthy"},
                {"component": "github", "state": "healthy"},
            ]
        }
        manifest = MagicMock(version="1.4.0")
        with (
            patch(
                "codexy_runtime_tools.monolith_migration_host.load_component_manifest",
                return_value=manifest,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.run_pre_session"
            ) as pre,
            patch(
                "codexy_runtime_tools.monolith_migration_host.run_github_pre_session"
            ) as github,
            patch(
                "codexy_runtime_tools.monolith_migration_host.run_operation",
                return_value={"outcome": "completed"},
            ) as operation,
            patch(
                "codexy_runtime_tools.monolith_migration_host.doctor",
                return_value=report,
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.status",
                return_value={"installed_components": ["core", "github"], "errors": []},
            ),
        ):
            activate(Path("/home"), Path("/codex"), lambda _: None, plan)

        pre.assert_called_once()
        github.assert_called_once()
        self.assertEqual(operation.call_args.args[:2], ("install", ("core", "github")))
        self.assertTrue(operation.call_args.kwargs["lock_held"])

    def test_activation_rejects_an_unexpected_leftover_component(self) -> None:
        plan = MigrationPlan("ready", "1.3.0", "1.4.0", ("core",), None, "")
        with (
            patch(
                "codexy_runtime_tools.monolith_migration_host.load_component_manifest",
                return_value=MagicMock(version="1.4.0"),
            ),
            patch("codexy_runtime_tools.monolith_migration_host.run_pre_session"),
            patch(
                "codexy_runtime_tools.monolith_migration_host.run_operation",
                return_value={"outcome": "completed"},
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.doctor",
                return_value={
                    "component_health": [{"component": "core", "state": "healthy"}]
                },
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.status",
                return_value={"installed_components": ["core", "github"], "errors": []},
            ),
        ):
            with self.assertRaisesRegex(RuntimeError, "did not converge"):
                activate(Path("/home"), Path("/codex"), lambda _: None, plan)

    def test_rollback_requires_exact_legacy_recovery(self) -> None:
        snapshot = MagicMock()
        snapshot.capture.return_value = snapshot
        journal = MigrationJournal("1.3.0", "1.4.0", ("core", "github"), snapshot)
        events: list[str] = []
        with (
            patch(
                "codexy_runtime_tools.monolith_migration_host.remove_split_components",
                side_effect=lambda *_: events.append("remove-extensions"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.reconcile_official_marketplace_root",
                side_effect=lambda *_: events.append("repin-source"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.run_pre_session",
                side_effect=lambda *_args, **_kwargs: events.append("restore-core"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.classify_monolith",
                return_value=MagicMock(state="supported-unmodified"),
            ),
            patch(
                "codexy_runtime_tools.monolith_migration_host.require_split_extensions_absent"
            ),
        ):
            rollback(
                Path("/home"),
                Path("/codex"),
                lambda _: None,
                journal,
                lambda *_: (Path("/legacy"), "1.3.0"),
            )

        self.assertEqual(events, ["remove-extensions", "repin-source", "restore-core"])
        snapshot.restore.assert_called_once()


if __name__ == "__main__":
    unittest.main()
