from __future__ import annotations

import unittest
from pathlib import Path


HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"


class WatcherInterpreterStartupTests(unittest.TestCase):
    def test_version_gate_uses_the_event_helper_before_the_hook_body(self):
        source = (HOOKS / "codexy_watcher_interrupt.py").read_text(encoding="utf-8")
        events = (HOOKS / "codexy_watcher_interrupt_events.py").read_text(
            encoding="utf-8"
        )
        self.assertIn("UNSUPPORTED_INTERPRETER_EXIT = 125", events)
        self.assertIn("raise SystemExit(UNSUPPORTED_INTERPRETER_EXIT)", source)
        gate = source.index("if sys.version_info < (3, 10):")
        self.assertLess(gate, source.index("def main"))
        self.assertIn("read(1024 * 1024 + 1)", source)
