"""Reuse, identity, corruption, and path-safety scenarios."""

from __future__ import annotations

import json
import os
from pathlib import Path
from unittest.mock import patch

from .common import BatchChangeResumeCase, COMMAND_SOURCE, ResumeError


class BatchChangeResumeReuseTests(BatchChangeResumeCase):
    def test_reuses_jointly_verified_result_without_invoking_commands_again(
        self,
    ) -> None:
        first = self._run()
        self.assertEqual(first["status"], "completed")
        self.assertEqual(first["items"][0]["resolution"], "rerun")
        self.assertEqual(first["items"][0]["status"], "succeeded")
        self.assertEqual(first["items"][0]["invocations"], 1)

        second = self._run()
        self.assertEqual(second["status"], "completed")
        self.assertEqual(second["items"][0]["resolution"], "reuse")
        self.assertEqual(second["items"][0]["invocations"], 1)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 1)
        self.assertEqual(self._log_count(self.validation_log, "validation"), 1)

    def test_changed_original_is_a_conflict_and_never_reuses_old_output(self) -> None:
        self._run()
        self.original.write_text("changed\n", encoding="utf-8")
        result = self._run()
        self.assertEqual(result["status"], "conflict")
        self.assertEqual(result["items"][0]["resolution"], "conflict")
        self.assertEqual(result["items"][0]["reason"], "original-changed")
        self.assertEqual(self._log_count(self.transform_log, "transform"), 1)

    def test_changed_command_validation_or_environment_forces_a_rerun(self) -> None:
        with patch.dict(os.environ, {"BATCH_RESUME_TEST_ENV": "first"}):
            self._run()
        changed = self._item(validation_timeout=4)
        with patch.dict(os.environ, {"BATCH_RESUME_TEST_ENV": "second"}):
            result = self._run(changed)
        self.assertEqual(result["items"][0]["resolution"], "rerun")
        self.assertEqual(result["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)
        self.assertEqual(self._log_count(self.validation_log, "validation"), 2)

    def test_changed_command_dependency_forces_a_rerun(self) -> None:
        self._run()
        self.command.write_text(
            COMMAND_SOURCE + "\n# dependency changed\n", encoding="utf-8"
        )
        result = self._run()
        self.assertEqual(result["items"][0]["resolution"], "rerun")
        self.assertEqual(result["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)
        self.assertEqual(self._log_count(self.validation_log, "validation"), 2)

    def test_incomplete_saved_fingerprints_force_a_rerun(self) -> None:
        first = self._run()
        artifact = Path(first["items"][0]["result"]["output"]["path"])
        artifact.write_text("corrupted\n", encoding="utf-8")
        state_file = self._state_file()
        state = json.loads(state_file.read_text(encoding="utf-8"))
        saved_result = state["items"]["item"]["result"]
        saved_result["artifact_state"] = {}
        saved_result["runner"]["output"]["state"] = {}
        state_file.write_text(json.dumps(state), encoding="utf-8")

        second = self._run()

        self.assertEqual(second["items"][0]["resolution"], "rerun")
        self.assertEqual(second["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)

    def test_changed_extensionless_transformer_with_option_forces_a_rerun(
        self,
    ) -> None:
        extensionless = self.root / "commands"
        extensionless.write_text(COMMAND_SOURCE, encoding="utf-8")
        item = self._item()
        item["transform"]["argv"][1:2] = ["-u", str(extensionless)]
        self._run(item)
        extensionless.write_text(
            COMMAND_SOURCE + "\nraise SystemExit(42)\n", encoding="utf-8"
        )

        second = self._run(item)

        self.assertEqual(second["items"][0]["resolution"], "rerun")
        self.assertEqual(second["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)

    def test_changed_extensionless_validator_with_option_forces_a_rerun(
        self,
    ) -> None:
        extensionless = self.root / "commands"
        extensionless.write_text(COMMAND_SOURCE, encoding="utf-8")
        item = self._item()
        item["validations"][0]["argv"][1:2] = ["-u", str(extensionless)]
        self._run(item)
        extensionless.write_text(
            COMMAND_SOURCE + "\nraise SystemExit(42)\n", encoding="utf-8"
        )

        second = self._run(item)

        self.assertEqual(second["items"][0]["resolution"], "rerun")
        self.assertEqual(second["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)

    def test_corrupt_state_fails_closed_without_running_a_command(self) -> None:
        self._run()
        state_file = self._state_file()
        state_file.write_text("{", encoding="utf-8")
        with self.assertRaises(ResumeError):
            self._run()
        self.assertEqual(self._log_count(self.transform_log, "transform"), 1)

    def test_missing_artifact_forces_rerun_and_does_not_reuse_leftover_state(
        self,
    ) -> None:
        first = self._run()
        artifact = Path(first["items"][0]["result"]["output"]["path"])
        artifact.unlink()
        second = self._run()
        self.assertEqual(second["items"][0]["resolution"], "rerun")
        self.assertEqual(second["items"][0]["invocations"], 2)
        self.assertTrue(Path(second["items"][0]["result"]["output"]["path"]).is_file())
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)

    def test_symlinked_state_root_is_rejected_without_running_a_command(self) -> None:
        canonical_root = self.root.resolve()
        target = canonical_root / "state-target"
        target.mkdir()
        alias = canonical_root / "state-link"
        alias.symlink_to(target, target_is_directory=True)
        with self.assertRaises(ResumeError):
            self._run(state_root=alias)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 0)
