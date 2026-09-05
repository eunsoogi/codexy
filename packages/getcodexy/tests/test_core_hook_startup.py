from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Mapping
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
HOOKS = ROOT / "plugins/codexy/hooks"
ENTRYPOINTS = (
    "codexy-child-thread-creation.py",
    "codexy-subagent-ownership.py",
    "codexy-thread-delivery.py",
)


class CoreHookStartupTests(unittest.TestCase):
    def test_version_gate_is_before_policy_import_and_launchers_do_not_probe_with_c(
        self,
    ):
        for entrypoint in ENTRYPOINTS:
            source = (HOOKS / entrypoint).read_text(encoding="utf-8")
            gate = source.index("UNSUPPORTED_INTERPRETER_EXIT = 125")
            policy_import = source.index("from codexy_policy")
            self.assertLess(gate, policy_import, entrypoint)
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

    @unittest.skipUnless(os.name != "nt", "POSIX launcher coverage")
    def test_allowed_paths_remain_zero_bytes_and_denials_keep_event_shapes(self):
        allowed = (
            (
                "codexy-thread-delivery.sh",
                "mcp__codex_app__send_message_to_thread",
                {"threadId": "parent", "model": "gpt-6-astra", "thinking": "medium"},
            ),
            (
                "codexy-child-thread-creation.sh",
                "mcp__codex_app__create_thread",
                {"model": "gpt-5.6-luna", "thinking": "max"},
            ),
            (
                "codexy-subagent-ownership.sh",
                "multi_agent_v1__spawn_agent",
                {"agent_type": "codexy-cartographer", "message": "Map files only."},
            ),
        )
        for launcher, tool, tool_input in allowed:
            with self.subTest(launcher=launcher):
                payload = _payload("PreToolUse", tool, tool_input)
                self.assertEqual(
                    self._run(HOOKS.parent, launcher, "PreToolUse", payload), b""
                )

        for launcher, prefix, tool in (
            (
                "codexy-thread-delivery.sh",
                "CODEXY_THREAD_DELIVERY_",
                "mcp__codex_app__send_message_to_thread",
            ),
            (
                "codexy-child-thread-creation.sh",
                "CODEXY_CHILD_THREAD_CREATION_",
                "mcp__codex_app__create_thread",
            ),
            (
                "codexy-subagent-ownership.sh",
                "CODEXY_SUBAGENT_OWNERSHIP_",
                "multi_agent_v1__spawn_agent",
            ),
        ):
            with self.subTest(launcher=launcher):
                for event in ("PreToolUse", "PermissionRequest"):
                    output = self._run(
                        HOOKS.parent,
                        launcher,
                        event,
                        _payload(event, "wrong_tool", {}),
                    )
                    value = json.loads(output)
                    specific = value["hookSpecificOutput"]
                    self.assertEqual(specific["hookEventName"], event)
                    decision = (
                        specific["decision"]
                        if event == "PermissionRequest"
                        else specific
                    )
                    reason = (
                        decision["message"]
                        if event == "PermissionRequest"
                        else decision["permissionDecisionReason"]
                    )
                    self.assertEqual(
                        decision["behavior"]
                        if event == "PermissionRequest"
                        else decision["permissionDecision"],
                        "deny",
                    )
                    self.assertTrue(reason.startswith(prefix + "ENVELOPE"), reason)

        oversized = self._run(
            HOOKS.parent,
            "codexy-thread-delivery.sh",
            "PreToolUse",
            b"x" * (1024 * 1024 + 1),
        )
        self.assertIn(b"CODEXY_THREAD_DELIVERY_ENVELOPE", oversized)

    @unittest.skipUnless(os.name != "nt", "POSIX launcher coverage")
    def test_retry_reuses_one_bounded_input_after_unsupported_interpreter(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "plugin"
            root.mkdir()
            shutil.copytree(HOOKS, root / "hooks")
            unsupported = root / "unsupported-interpreter"
            unsupported.write_text("#!/bin/sh\nexit 125\n", encoding="utf-8")
            unsupported.chmod(0o755)
            supported = root / "supported-interpreter"
            supported.symlink_to(Path(sys.executable))
            runtime = root / "hooks/codexy-hook-runtime.sh"
            runtime.write_text(
                runtime.read_text(encoding="utf-8").replace(
                    "for candidate in /usr/local/bin/python3 /usr/bin/python3; do",
                    f"for candidate in {unsupported} {supported}; do",
                ),
                encoding="utf-8",
            )
            (root / "hooks/codexy-child-thread-creation.py").write_text(
                "import os\n"
                "import sys\n"
                "from pathlib import Path\n"
                "if sys.version_info < (3, 10):\n"
                "    raise SystemExit(125)\n"
                "Path(os.environ['PLUGIN_ROOT'], 'replayed.bin').write_bytes("
                "sys.stdin.buffer.read())\n",
                encoding="utf-8",
            )
            payload = b"payload\x00with-bytes"
            self.assertEqual(
                self._run(
                    root,
                    "codexy-child-thread-creation.sh",
                    "PreToolUse",
                    payload,
                ),
                b"",
            )
            self.assertEqual((root / "replayed.bin").read_bytes(), payload)

    @unittest.skipUnless(os.name != "nt", "POSIX launcher coverage")
    def test_policy_failure_is_not_retried_and_falls_back_event_natively(self):
        for event in ("PreToolUse", "PermissionRequest"):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary) / "plugin"
                root.mkdir()
                shutil.copytree(HOOKS, root / "hooks")
                (root / "hooks/codexy-child-thread-creation.py").write_text(
                    "import os\n"
                    "from pathlib import Path\n"
                    "attempts = Path(os.environ['PLUGIN_ROOT'], 'attempts')\n"
                    "attempts.write_text(str(int(attempts.read_text() or '0') + 1))\n"
                    "raise RuntimeError('fixture failure')\n",
                    encoding="utf-8",
                )
                (root / "attempts").write_text("0", encoding="utf-8")
                output = self._run(
                    root,
                    "codexy-child-thread-creation.sh",
                    event,
                    b"fixture",
                )
                value = json.loads(output)
                specific = value["hookSpecificOutput"]
                self.assertEqual(specific["hookEventName"], event)
                self.assertEqual((root / "attempts").read_text(encoding="utf-8"), "1")

    @unittest.skipUnless(os.name != "nt", "POSIX launcher coverage")
    def test_reserved_exit_from_policy_is_normalized_without_retry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "plugin"
            root.mkdir()
            shutil.copytree(HOOKS, root / "hooks")
            (root / "hooks/codexy_policy/child_thread_creation.py").write_text(
                "import os\n"
                "from pathlib import Path\n"
                "def forbidden(_request):\n"
                "    attempts = Path(os.environ['PLUGIN_ROOT'], 'attempts')\n"
                "    attempts.write_text(str(int(attempts.read_text() or '0') + 1))\n"
                "    raise SystemExit(125)\n",
                encoding="utf-8",
            )
            (root / "hooks/codexy_policy/envelope.py").write_text(
                "import os\n"
                "from pathlib import Path\n"
                "def evaluate(_event, _payload, _tools, _diagnostic, forbidden):\n"
                "    forbidden(None)\n"
                "    return b''\n",
                encoding="utf-8",
            )
            (root / "attempts").write_text("0", encoding="utf-8")
            output = self._run(
                root,
                "codexy-child-thread-creation.sh",
                "PreToolUse",
                b"fixture",
            )
            self.assertIn(b"CODEXY_CHILD_THREAD_CREATION_RUNTIME", output)
            self.assertEqual((root / "attempts").read_text(encoding="utf-8"), "1")

    def _run(self, root: Path, launcher: str, event: str, payload: bytes) -> bytes:
        result = subprocess.run(
            [str(root / "hooks" / launcher), event],
            input=payload,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={**os.environ, "PLUGIN_ROOT": str(root)},
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        self.assertEqual(result.stderr, b"")
        return result.stdout


def _payload(event: str, tool: str, tool_input: Mapping[str, object]) -> bytes:
    return json.dumps(
        {"hook_event_name": event, "tool_name": tool, "tool_input": tool_input}
    ).encode()


if __name__ == "__main__":
    unittest.main()
