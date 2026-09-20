"""Competing-executor locking scenario."""

from __future__ import annotations

import json
import signal
import subprocess

from .common import BatchChangeResumeCase, ROOT


class BatchChangeResumeConcurrencyTests(BatchChangeResumeCase):
    def test_competing_executors_report_conflict_while_first_holds_lock(self) -> None:
        manifest = self._manifest(
            self._item(transform="slow-copy", transform_delay=1.5)
        )
        first_process = subprocess.Popen(
            self._cli_args(manifest),
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self._wait_for_state()
        second = subprocess.run(
            self._cli_args(manifest),
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(second.returncode, 3, second.stderr)
        self.assertEqual(json.loads(second.stdout)["status"], "conflict")
        first_process.send_signal(signal.SIGINT)
        stdout, stderr = first_process.communicate(timeout=10)
        self.assertEqual(stderr, "", stderr)
        self.assertEqual(first_process.returncode, 0)
        self.assertEqual(json.loads(stdout)["status"], "interrupted")
