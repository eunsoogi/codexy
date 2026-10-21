"""Verify the shared core hook runtime rejects retired lifecycle adapters."""

from __future__ import annotations

import os
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RUNTIME = ROOT / "plugins/codexy/hooks/codexy-hook-runtime.sh"


class CoreHookRuntimeTests(unittest.TestCase):
    @unittest.skipUnless(os.name != "nt", "POSIX shared runtime coverage")
    def test_retired_watcher_entrypoint_and_lifecycle_events_are_rejected(self):
        cases = (
            ("codexy_watcher_interrupt.py", "Interrupt"),
            ("codexy-thread-delivery.py", "PostToolUse"),
            ("codexy-thread-delivery.py", "Interrupt"),
            ("codexy-thread-delivery.py", "UserPromptSubmit"),
        )
        for entrypoint, event in cases:
            with self.subTest(entrypoint=entrypoint, event=event):
                result = subprocess.run(
                    ["/bin/sh", str(RUNTIME), entrypoint, event],
                    input=b"",
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    check=False,
                )
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, b"")


if __name__ == "__main__":
    unittest.main()
