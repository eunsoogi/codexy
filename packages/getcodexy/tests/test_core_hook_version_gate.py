"""Verify interpreter gates and bounded hook launchers."""

import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"
ENTRYPOINTS = (
    "codexy-child-thread-creation.py",
    "codexy-subagent-ownership.py",
    "codexy-thread-delivery.py",
)


class CoreHookVersionGateTests(unittest.TestCase):
    def test_version_gate_is_before_policy_import_and_launchers_do_not_probe_with_c(
        self,
    ):
        for entrypoint in ENTRYPOINTS:
            source = (HOOKS / entrypoint).read_text(encoding="utf-8")
            gate = source.index("UNSUPPORTED_INTERPRETER_EXIT = 125")
            anchors = [source.index("def main")]
            if "from codexy_policy" in source:
                anchors.append(source.index("from codexy_policy"))
            self.assertLess(gate, min(anchors), entrypoint)
            self.assertIn("read(1024 * 1024 + 1)", source, entrypoint)

        runtime = (HOOKS / "codexy-hook-runtime.sh").read_text(encoding="utf-8")
        self.assertIn("125)", runtime)
        self.assertNotIn("-c \\\n", runtime)
        for launcher in (
            "codexy-child-thread-creation.cmd",
            "codexy-subagent-ownership.cmd",
            "codexy-thread-delivery.cmd",
        ):
            source = (HOOKS / launcher).read_text(encoding="utf-8")
            self.assertNotIn(" -c ", source)
            self.assertNotIn(">", source)
            self.assertNotIn("CODEXY_HOOK_OUTPUT", source)
            self.assertIn("%SystemRoot%\\py.exe", source)
