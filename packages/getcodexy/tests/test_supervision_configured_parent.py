"""First owner/prepare and hook use the explicitly host-configured private root."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"
CREATE_INPUT = {"model": "m", "thinking": "medium", "prompt": "synthetic"}
EVENT = {
    "hook_event_name": "PreToolUse",
    "tool_name": "mcp__codex_app__create_thread",
    "session_id": "parent",
    "tool_use_id": "call",
    "tool_input": CREATE_INPUT,
}


class ConfiguredParentTests(unittest.TestCase):
    def invoke(self, name, *args, root=None, value=None):
        env = os.environ.copy()
        env.pop("PLUGIN_DATA", None)
        if root is not None:
            env["PLUGIN_DATA"] = root
        completed = subprocess.run(
            [sys.executable, "-I", "-B", str(HOOKS / name), *args],
            input=json.dumps(value),
            text=True,
            capture_output=True,
            env=env,
            check=False,
        )
        return completed.returncode, json.loads(completed.stdout)

    def test_first_request_uses_shared_root_and_pending_remains_reserved(self):
        with tempfile.TemporaryDirectory() as root:
            owner = (
                "codexy_supervision_owner.py",
                "--parent",
                "parent",
                "bootstrap",
                "--confirmed-complete",
            )
            prepare = (
                "codexy_supervision_prepare.py",
                "--parent",
                "parent",
                "--issue",
                "issue",
            )
            self.assertEqual(
                self.invoke(*owner, root=root, value=[]),
                (0, {"result": "reconciled"}),
            )
            self.assertEqual(
                self.invoke(*prepare, root=root, value=CREATE_INPUT)[1]["result"],
                "reserved",
            )
            hook = "codexy_supervision_capacity.py"
            self.assertEqual(
                self.invoke(hook, "--event", "PreToolUse", root=root, value=EVENT),
                (0, {}),
            )
            post = {
                **EVENT,
                "hook_event_name": "PostToolUse",
                "tool_response": {"structuredContent": {"clientThreadId": "pending"}},
            }
            self.assertEqual(
                self.invoke(hook, "--event", "PostToolUse", root=root, value=post),
                (0, {}),
            )
            self.assertEqual(
                self.invoke(*prepare, root=root, value=CREATE_INPUT)[1]["result"],
                "existing",
            )
            _, denied = self.invoke(
                hook,
                "--event",
                "PreToolUse",
                root=root,
                value={**EVENT, "tool_use_id": "repeat"},
            )
            self.assertEqual(denied["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_missing_or_disagreeing_roots_never_create_state(self):
        with tempfile.TemporaryDirectory() as root:
            conflicting = str(Path(root) / "unavailable")
            for name, args, value in (
                (
                    "codexy_supervision_owner.py",
                    ("bootstrap", "--confirmed-complete"),
                    [],
                ),
                ("codexy_supervision_prepare.py", ("--issue", "issue"), CREATE_INPUT),
            ):
                head = (name, "--parent", "parent")
                missing, missing_body = self.invoke(*head, *args, value=value)
                self.assertNotEqual(missing, 0)
                self.assertNotIn(root, json.dumps(missing_body))
                conflict, conflict_body = self.invoke(
                    *head, "--plugin-data", conflicting, *args, root=root, value=value
                )
                self.assertNotEqual(conflict, 0)
                self.assertNotIn(root, json.dumps(conflict_body))
            self.assertEqual(list(Path(root).iterdir()), [])
