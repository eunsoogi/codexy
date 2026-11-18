"""Protocol transaction tests; registration doubles do not prove native installation."""

import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_transaction_state import read_journal
from codexy_runtime_tools.local_upgrade_state import read, tree_digest
from packages.getcodexy.tests.local_upgrade_fixture import UpgradeHost, MANIFEST


class LocalUpgradeTransactionTests(unittest.TestCase):
    def test_upgrade_preserves_selection_settings_and_old_archive(self):
        for selection in (
            ("core",),
            ("core", "github"),
            ("core", "github", "devtools"),
        ):
            with self.subTest(selection=selection), UpgradeHost(selection) as host:
                old_digest = tree_digest(host.old)
                receipt = host.operate()
                self.assertEqual(receipt["outcome"], "completed")
                self.assertEqual(receipt["selection_after"], list(selection))
                self.assertEqual(host.marketplace, host.target)
                self.assertEqual(set(host.records), set(selection))
                self.assertTrue(
                    all(
                        root == host.target and version == MANIFEST.version
                        for root, version in host.records.values()
                    )
                )
                self.assertEqual(tree_digest(host.old), old_digest)
                self.assertEqual(
                    (host.home / "config.toml").read_bytes(), host.settings
                )
                self.assertIsNone(read_journal(host.home))
                self.assertIsNone(read(host.home))

    def test_requested_subset_still_upgrades_all_selected_components(self):
        with UpgradeHost() as host:
            receipt = host.operate(requested=("core",))
            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(set(host.records), {"core", "github", "devtools"})
            self.assertTrue(
                all(version == MANIFEST.version for _, version in host.records.values())
            )

    def test_partial_add_failure_restores_previous_root_and_versions(self):
        with UpgradeHost() as host:
            old_digest = tree_digest(host.old)
            host.fail_add = "github"
            receipt = host.operate()
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(host.marketplace, host.old)
            self.assertTrue(
                all(
                    root == host.old and version == "2.2.4"
                    for root, version in host.records.values()
                )
            )
            self.assertEqual(tree_digest(host.old), old_digest)
            self.assertEqual((host.home / "config.toml").read_bytes(), host.settings)
            self.assertIsNone(read(host.home))
            # Identical operation IDs replay the durable result without another mutation.
            mutations = list(host.mutations)
            self.assertEqual(host.operate(), receipt)
            self.assertEqual(host.mutations, mutations)
            self.assertEqual(
                host.operate(identifier="op-retry-local")["outcome"], "completed"
            )

    def test_registration_failure_after_remove_recovers_old_binding(self):
        with UpgradeHost() as host:
            host.fail_marketplace_add = True
            receipt = host.operate()
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertTrue(host.present)
            self.assertEqual(host.marketplace, host.old)

    def test_mixed_unjournaled_identity_rejected_without_mutations(self):
        with UpgradeHost() as host:
            host.records["github"] = (host.target, MANIFEST.version)
            receipt = host.operate()
            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(host.mutations, [])
            host.acquire.assert_not_called()

    def test_target_verification_failure_preserves_old_state(self):
        with UpgradeHost() as host:
            host.acquire.side_effect = ValueError("invalid release attestation")
            receipt = host.operate()
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(host.mutations, [])
            self.assertEqual(host.marketplace, host.old)

    def test_registration_failure_is_not_reported_as_success(self):
        with (
            UpgradeHost() as host,
            patch(
                "codexy_runtime_tools.component_registration_health.sync_agents",
                side_effect=RuntimeError("registration gate failed"),
            ),
        ):
            receipt = host.operate()
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(host.marketplace, host.old)

    def test_interrupted_partial_upgrade_rolls_back_on_restart(self):
        with UpgradeHost() as host:
            host.fail_add = "github"
            # Simulate process death before error handling can restore the prior state.
            with patch(
                "codexy_runtime_tools.component_lifecycle_operation._rollback_or_raise",
                side_effect=SystemExit("simulated process death"),
            ):
                with self.assertRaises(SystemExit):
                    host.operate()
            self.assertIsNotNone(read(host.home))
            self.assertIsNotNone(read_journal(host.home))
            self.assertEqual(host.records["core"][1], MANIFEST.version)
            self.assertEqual(host.records["github"][1], "2.2.4")
            receipt = host.operate()
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(host.marketplace, host.old)
            self.assertTrue(
                all(version == "2.2.4" for _, version in host.records.values())
            )
            self.assertIsNone(read(host.home))
            self.assertIsNone(read_journal(host.home))

    def test_old_archive_tamper_blocks_rollback_execution(self):
        with UpgradeHost() as host:
            host.fail_add = "github"
            with patch(
                "codexy_runtime_tools.component_lifecycle_operation._rollback_or_raise",
                side_effect=SystemExit("simulated process death"),
            ):
                with self.assertRaises(SystemExit):
                    host.operate()
            (host.old / "injected.py").write_text("untrusted")
            mutations = list(host.mutations)
            with self.assertRaisesRegex(ValueError, "prior local marketplace changed"):
                host.operate()
            self.assertEqual(host.mutations, mutations)
            self.assertIsNotNone(read(host.home))
            self.assertIsNotNone(read_journal(host.home))

    def test_target_tamper_before_add_rolls_back_without_touching_old_archive(self):
        with UpgradeHost() as host:
            digest = tree_digest(host.old)

            def tamper(component):
                if host.marketplace == host.target:
                    (host.target / "injected_hook.py").write_text("untrusted")

            host.before_add = tamper
            receipt = host.operate()
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(host.marketplace, host.old)
            self.assertEqual(tree_digest(host.old), digest)
            self.assertTrue(
                all(version == "2.2.4" for _, version in host.records.values())
            )

    def test_committed_target_tamper_blocks_activation_on_restart(self):
        with UpgradeHost() as host:
            with patch(
                "codexy_runtime_tools.component_lifecycle_recovery.finish_committed",
                side_effect=SystemExit("simulated process death"),
            ):
                with self.assertRaises(SystemExit):
                    host.operate()
            self.assertEqual(read_journal(host.home).phase, "committed")
            (host.target / "injected_hook.py").write_text("untrusted")
            mutations = list(host.mutations)
            with self.assertRaisesRegex(
                ValueError, "verified target marketplace changed"
            ):
                host.operate()
            self.assertEqual(host.mutations, mutations)
            self.assertIsNotNone(read(host.home))

    def test_pending_upgrade_clears_only_dead_registration_locks(self):
        for owner in ("dead", "live", "unknown"):
            with self.subTest(owner=owner), UpgradeHost() as host:
                host.fail_add = "github"
                with patch(
                    "codexy_runtime_tools.component_lifecycle_operation._rollback_or_raise",
                    side_effect=SystemExit("simulated process death"),
                ):
                    with self.assertRaises(SystemExit):
                        host.operate()
                lock = host.home / ".codexy-agent-registration.lock"
                lock.write_text("12345")
                mutations = list(host.mutations)
                with patch(
                    "codexy_runtime_tools.component_transaction_state.registration_owner_state",
                    return_value=owner,
                ):
                    if owner == "dead":
                        self.assertEqual(host.operate()["outcome"], "rolled-back")
                        self.assertFalse(lock.exists())
                        self.assertEqual(host.marketplace, host.old)
                    else:
                        with self.assertRaisesRegex(
                            RuntimeError, "another Codexy agent registration"
                        ):
                            host.operate()
                        self.assertTrue(lock.exists())
                        self.assertEqual(host.mutations, mutations)
                        self.assertIsNotNone(read_journal(host.home))
