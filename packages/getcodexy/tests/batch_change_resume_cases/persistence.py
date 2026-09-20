"""Atomic persistence interruption scenario."""

from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

from .common import BatchChangeResumeCase, RESUME_SCRIPTS, ROOT


class BatchChangeResumePersistenceTests(BatchChangeResumeCase):
    def test_interruption_before_state_replace_leaves_prior_checkpoint_and_reruns(
        self,
    ) -> None:
        manifest = self._manifest()
        count_file = Path(self.temporary.name) / "persist-count.txt"
        ready_file = Path(self.temporary.name) / "persist-ready"
        helper = """
import sys, time
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from resume import resume_from_path

count = Path(sys.argv[6])
ready = Path(sys.argv[7])

def pause(phase, path):
    if phase != "before_replace":
        return
    number = int(count.read_text(encoding="utf-8")) + 1 if count.exists() else 1
    count.write_text(str(number), encoding="utf-8")
    if number == 3:
        ready.write_text("ready", encoding="utf-8")
        while True:
            time.sleep(0.05)

resume_from_path(
    sys.argv[2],
    sys.argv[3],
    state_root=sys.argv[4],
    results_root=sys.argv[5],
    persistence_hook=pause,
)
"""
        process = subprocess.Popen(
            [
                sys.executable,
                "-c",
                helper,
                str(RESUME_SCRIPTS),
                str(self.root),
                str(manifest),
                str(self.root / ".codexy-batch-state"),
                str(self.root / ".codexy-batch-results"),
                str(count_file),
                str(ready_file),
            ],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        deadline = time.monotonic() + 5
        while not ready_file.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertTrue(ready_file.exists())
        process.kill()
        process.communicate(timeout=10)
        state_file = self._state_file()
        self.assertEqual(
            json.loads(state_file.read_text())["items"]["item"]["status"],
            "in-progress",
        )

        result = self._cli_result(manifest)[1]
        self.assertEqual(result["items"][0]["resolution"], "rerun")
        self.assertEqual(result["items"][0]["invocations"], 2)
        self.assertEqual(self._log_count(self.transform_log, "transform"), 2)
