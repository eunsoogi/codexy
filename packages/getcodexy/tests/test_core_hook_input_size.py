"""Verify the thread-delivery launcher honors its bounded hook-input size."""

import os
import subprocess
import unittest
from pathlib import Path

from core_hook_test_support import sized_thread_delivery_payload


ROOT = Path(__file__).resolve().parents[3]
LAUNCHER = ROOT / "plugins/codexy/hooks/codexy-thread-delivery.sh"
MAX_BYTES = 1024 * 1024


class CoreHookInputSizeTests(unittest.TestCase):
    @unittest.skipUnless(os.name != "nt", "POSIX launcher coverage")
    def test_valid_delivery_at_limit_admits_and_oversized_delivery_denies(self) -> None:
        for event in ("PermissionRequest", "PreToolUse"):
            with self.subTest(event=event):
                self.assertEqual(self._run(event, MAX_BYTES), b"")
                self.assertIn(
                    b"CODEXY_THREAD_DELIVERY_ENVELOPE",
                    self._run(event, MAX_BYTES + 1),
                )

    def _run(self, event: str, size: int) -> bytes:
        result = subprocess.run(
            [str(LAUNCHER), event],
            input=sized_thread_delivery_payload(size, event),
            capture_output=True,
            env={**os.environ, "PLUGIN_ROOT": str(ROOT / "plugins/codexy")},
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        self.assertEqual(result.stderr, b"")
        return result.stdout
