"""Real transform and validation interruption scenarios."""

from __future__ import annotations

import signal
import subprocess
import time

from .common import BatchChangeResumeCase, ROOT


class BatchChangeResumeInterruptionTests(BatchChangeResumeCase):
    def test_transform_interruption_persists_incomplete_item_then_reruns(self) -> None:
        manifest = self._manifest(
            self._item(transform="slow-copy", transform_delay=2.0)
        )
        first = subprocess.Popen(
            self._cli_args(manifest),
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        state_file = self._wait_for_state()
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if self._log_count(self.transform_log, "transform"):
                break
            time.sleep(0.02)
        self.assertTrue(self._log_count(self.transform_log, "transform"))
        first.send_signal(signal.SIGINT)
        stdout, stderr = first.communicate(timeout=10)
        self.assertEqual(stderr, "", stderr)
        self.assertEqual(json_status(stdout), "interrupted")
        self.assertEqual(load_state_status(state_file), "failed")

        second = self._cli_result(manifest)[1]
        self.assertEqual(second["items"][0]["resolution"], "rerun")
        self.assertEqual(second["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)

    def test_validation_interruption_persists_incomplete_item_then_reruns(self) -> None:
        manifest = self._manifest(
            self._item(validation="slow-validate", validation_delay=2.0)
        )
        first = subprocess.Popen(
            self._cli_args(manifest),
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        state_file = self._wait_for_state()
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if self._log_count(self.validation_log, "validation"):
                break
            time.sleep(0.02)
        self.assertTrue(self._log_count(self.validation_log, "validation"))
        first.send_signal(signal.SIGINT)
        stdout, stderr = first.communicate(timeout=10)
        self.assertEqual(stderr, "", stderr)
        self.assertEqual(json_status(stdout), "interrupted")
        self.assertEqual(load_state_status(state_file), "failed")

        second = self._cli_result(manifest)[1]
        self.assertEqual(second["items"][0]["resolution"], "rerun")
        self.assertEqual(second["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)
        self.assertEqual(self._log_count(self.validation_log, "validation"), 2)


def json_status(stdout: str) -> str:
    import json

    return json.loads(stdout)["status"]


def load_state_status(path) -> str:
    import json

    return json.loads(path.read_text())["items"]["item"]["status"]
