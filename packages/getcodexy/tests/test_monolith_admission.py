from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.activation_transaction import ActivationSnapshot
from codexy_runtime_tools.monolith_baseline import BASELINES, Baseline, tree_digest
from codexy_runtime_tools.monolith_classifier import classify_monolith
from codexy_runtime_tools.monolith_migration_plan import plan_migration


class MonolithAdmissionTests(unittest.TestCase):
    def test_v1_3_baseline_is_a_frozen_complete_tree_fingerprint(self) -> None:
        self.assertEqual(
            BASELINES["1.3.0"].tree_sha256,
            "cdde46a96bf574f9b54a2445b6bb94c0841493148ec255bad220d4728a46ec0a",
        )

    def test_exact_supported_tree_is_admitted_but_modified_tree_is_rejected(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = _legacy_root(Path(directory))
            baseline = Baseline("9.9.9", tree_digest(root))
            with patch(
                "codexy_runtime_tools.monolith_classifier.BASELINES",
                {"9.9.9": baseline},
            ):
                self.assertEqual(classify_monolith(root).state, "supported-unmodified")
                (root / "skill.md").write_text("modified", encoding="utf-8")
                self.assertEqual(classify_monolith(root).state, "modified")

    def test_unknown_or_modified_monolith_fails_closed_with_a_specific_code(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = _legacy_root(Path(directory))
            ambiguous = plan_migration(root, "1.4.0", ())
            self.assertEqual(
                (ambiguous.outcome, ambiguous.error), ("rejected", "ambiguous-monolith")
            )

            baseline = Baseline("9.9.9", tree_digest(root))
            with patch(
                "codexy_runtime_tools.monolith_classifier.BASELINES",
                {"9.9.9": baseline},
            ):
                (root / "skill.md").write_text("modified", encoding="utf-8")
                modified = plan_migration(root, "1.4.0", ())
            self.assertEqual(
                (modified.outcome, modified.error), ("rejected", "modified-monolith")
            )

    def test_same_release_never_admits_a_repin(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = _legacy_root(Path(directory))
            baseline = Baseline("9.9.9", tree_digest(root))
            with patch(
                "codexy_runtime_tools.monolith_classifier.BASELINES",
                {"9.9.9": baseline},
            ):
                plan = plan_migration(root, "9.9.9", ())
            self.assertEqual(
                (plan.outcome, plan.error), ("rejected", "target-release-unavailable")
            )

    def test_default_and_explicit_selection_use_the_component_resolver(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = _legacy_root(Path(directory))
            baseline = Baseline("9.9.9", tree_digest(root))
            with patch(
                "codexy_runtime_tools.monolith_classifier.BASELINES",
                {"9.9.9": baseline},
            ):
                default = plan_migration(root, "10.0.0", ())
                explicit = plan_migration(root, "10.0.0", ("devtools",))
            self.assertEqual(default.selection, ("core", "github", "devtools"))
            self.assertEqual(explicit.selection, ("core", "devtools"))

    @unittest.skipIf(os.name == "nt", "POSIX link controls are exercised in CI")
    def test_link_hardlink_and_special_file_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = _legacy_root(Path(directory))
            baseline = Baseline("9.9.9", tree_digest(root))
            (root / "linked").symlink_to(root / "skill.md")
            with patch(
                "codexy_runtime_tools.monolith_classifier.BASELINES",
                {"9.9.9": baseline},
            ):
                self.assertEqual(classify_monolith(root).state, "ambiguous")
            (root / "linked").unlink()
            os.link(root / "skill.md", root / "hardlinked")
            with self.assertRaisesRegex(ValueError, "unsafe file"):
                tree_digest(root)

    @unittest.skipIf(os.name == "nt", "POSIX link controls are exercised in CI")
    def test_recovery_snapshot_refuses_linked_or_hardlinked_projection_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            agents = home / "agents" / "codexy"
            agents.mkdir(parents=True)
            agent = agents / "owner.toml"
            agent.write_text("owner", encoding="utf-8")
            os.link(agent, agents / "hardlinked.toml")
            with self.assertRaisesRegex(ValueError, "real directories"):
                ActivationSnapshot.capture(home)

            (agents / "hardlinked.toml").unlink()
            (agents / "linked.toml").symlink_to(agent)
            with self.assertRaisesRegex(ValueError, "refuses link"):
                ActivationSnapshot.capture(home)


def _legacy_root(parent: Path) -> Path:
    root = parent / "legacy"
    manifest = root / ".codex-plugin" / "plugin.json"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(
        json.dumps(
            {
                "name": "codexy",
                "repository": "https://github.com/eunsoogi/codexy",
                "version": "9.9.9",
            }
        ),
        encoding="utf-8",
    )
    (root / "skill.md").write_text("original", encoding="utf-8")
    return root


if __name__ == "__main__":
    unittest.main()
