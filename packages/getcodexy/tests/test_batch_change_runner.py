from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import time
import unittest
from pathlib import Path

RUNNER_SCRIPTS = (
    Path(__file__).parents[3]
    / "plugins/codexy/skills/engineering/scripts/batch_change_runner"
)
sys.path.insert(0, str(RUNNER_SCRIPTS))
from test_support import BatchChangeRunnerCase  # noqa: E402


@unittest.skipUnless(os.name == "posix", "runner requires POSIX process groups")
class BatchChangeRunnerTests(BatchChangeRunnerCase):
    def test_isolates_failures_and_preserves_validated_artifacts(self) -> None:
        self._write("first.txt", "second.txt", "third.txt")
        result = self._run_specs(
            ("failed-transform", "first.txt", "out/first.txt", "fail"),
            (
                "success",
                "second.txt",
                "out/second.txt",
                "copy",
                "second.txt",
                "out/second.txt",
            ),
            (
                "failed-validation",
                "third.txt",
                "out/third.txt",
                "bad",
                "third.txt",
                "out/third.txt",
            ),
        )
        by_id = {item["id"]: item for item in result["items"]}
        self.assertEqual(
            (result["status"], result["succeeded_count"], result["failed_count"]),
            ("completed", 1, 2),
        )
        self._reason(result, "nonzero-exit")
        self.assertEqual(by_id["failed-validation"]["failure"]["phase"], "validation")
        self.assertEqual(
            Path(by_id["success"]["output"]["path"]).read_text(), "SECOND.TXT"
        )
        self.assertTrue(by_id["success"]["output"]["validated"])
        self.assertEqual(
            [
                (self.root / name).read_text()
                for name in ("first.txt", "second.txt", "third.txt")
            ],
            ["first.txt", "second.txt", "third.txt"],
        )

    def test_contract_violations_and_original_races_abort(self) -> None:
        self._write("input.txt")
        outside = Path(self.temporary.name) / "stale"
        outside.mkdir()
        (outside / "file.txt").write_text("stale", encoding="utf-8")
        symlink = self._run_specs(
            ("ancestor", "input.txt", "out/file.txt", "symlink", "out", str(outside))
        )
        self._aborted(symlink, "contract-violation")
        self.assertEqual((outside / "file.txt").read_text(), "stale")

        self._write("second.txt")
        staged = self._run_specs(
            ("mutated-stage", "input.txt", "out.txt", "delete", "input.txt"),
            (
                "must-not-run",
                "second.txt",
                "other.txt",
                "copy",
                "second.txt",
                "other.txt",
            ),
        )
        self._aborted(staged, "contract-violation")
        self.assertEqual(staged["items"][0]["execution"]["transform"]["exit_code"], 7)
        self.assertEqual(staged["items"][1]["status"], "not-run")

        document = self._preview(
            self._item(
                "deleted",
                "input.txt",
                "deleted.txt",
                "copy",
                "input.txt",
                "deleted.txt",
            )
        )
        (self.root / "input.txt").unlink()
        self._aborted(self._run_document(document), "original-changed")

        self._write("first.txt", "second.txt")
        mutated = self._run_specs(
            (
                "mutates-other",
                "first.txt",
                "out/first.txt",
                "mutate",
                "first.txt",
                "out/first.txt",
                str(self.root / "second.txt"),
            ),
            (
                "must-not-run",
                "second.txt",
                "out/second.txt",
                "copy",
                "second.txt",
                "out/second.txt",
            ),
        )
        self._aborted(mutated, "original-changed")
        self.assertEqual(mutated["items"][1]["status"], "not-run")
        self.assertEqual((self.root / "second.txt").read_text(), "changed")

    def test_cli_runs_manifest_cancels_and_redacts_output(self) -> None:
        self._write("input.txt")
        success, payload = self._cli_result(
            self._manifest_specs(
                "success.json",
                ("success", "input.txt", "out.txt", "copy", "input.txt", "out.txt"),
            )
        )
        self.assertEqual(success.returncode, 0)
        self.assertEqual(payload["items"][0]["status"], "succeeded")
        self.assertEqual(
            Path(payload["items"][0]["output"]["path"]).read_text(), "INPUT.TXT"
        )
        self.assertEqual((self.root / "input.txt").read_text(), "input.txt")

        spam, payload = self._cli_result(
            self._manifest_specs(
                "spam.json", ("spam", "input.txt", "spam.txt", "spam")
            ),
            "--max-output-bytes",
            "128",
        )
        self._reason(payload, "output-limit")
        self.assertNotIn("SECRET_TOKEN", spam.stdout + spam.stderr)

        pid_file = Path(self.temporary.name) / "cli.pid"
        process = subprocess.Popen(
            self._cli(
                self._manifest_specs(
                    "cancel.json",
                    (
                        "cancel",
                        "input.txt",
                        "cancel.txt",
                        "spawn",
                        "input.txt",
                        "cancel.txt",
                        str(pid_file),
                        30,
                    ),
                )
            ),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        deadline = time.monotonic() + 2
        while not pid_file.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertTrue(pid_file.exists())
        process.send_signal(signal.SIGINT)
        stdout, stderr = process.communicate(timeout=5)
        self.assertEqual(process.returncode, 0, stderr)
        self.assertEqual(json.loads(stdout)["status"], "cancelled")
        self._assert_dead(pid_file)

    def test_timeout_kills_descendants(self) -> None:
        self._write("input.txt")
        pid_file = Path(self.temporary.name) / "timeout.pid"
        timeout = self._run_specs(
            (
                "timeout",
                "input.txt",
                "out.txt",
                "spawn",
                "input.txt",
                "out.txt",
                str(pid_file),
                0.15,
            )
        )
        self._reason(timeout, "timeout")
        self._assert_dead(pid_file)


if __name__ == "__main__":
    unittest.main()
