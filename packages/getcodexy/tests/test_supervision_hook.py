"""Exercise the hook entrypoint against a prepared private store."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"
previous_bytecode_setting = sys.dont_write_bytecode
sys.dont_write_bytecode = True
sys.path.insert(0, str(HOOKS))
from codexy_policy.supervision_storage import open_store

sys.dont_write_bytecode = previous_bytecode_setting


class SupervisionHookTests(unittest.TestCase):
    def test_pre_and_post_connect_only_exact_prepared_call(self):
        with tempfile.TemporaryDirectory() as root:
            tool_input = {"model": "m", "thinking": "medium", "prompt": "assignment"}
            store = open_store(root)
            preparation = subprocess.run(
                [
                    sys.executable,
                    "-I",
                    "-B",
                    str(HOOKS / "codexy-supervision-prepare.py"),
                    "--plugin-data",
                    root,
                    "--parent",
                    "parent",
                    "--issue",
                    "issue",
                ],
                input=json.dumps(tool_input),
                text=True,
                capture_output=True,
                check=True,
            )
            self.assertEqual(json.loads(preparation.stdout)["result"], "reserved")
            event = {
                "hook_event_name": "PreToolUse",
                "tool_name": "mcp__codex_app__create_thread",
                "session_id": "parent",
                "tool_use_id": "call",
                "tool_input": tool_input,
            }

            def invoke(name, data):
                result = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        "-B",
                        str(HOOKS / "codexy-supervision-capacity.py"),
                        "--event",
                        name,
                    ],
                    input=json.dumps(data),
                    text=True,
                    capture_output=True,
                    env={**os.environ, "PLUGIN_DATA": root},
                    check=True,
                )
                return json.loads(result.stdout)

            self.assertEqual(invoke("PreToolUse", event), {})
            denied = invoke("PreToolUse", {**event, "tool_use_id": "other"})
            self.assertEqual(denied["hookSpecificOutput"]["permissionDecision"], "deny")
            post = {
                **event,
                "hook_event_name": "PostToolUse",
                "tool_response": {"structuredContent": {"threadId": "child"}},
            }
            self.assertEqual(invoke("PostToolUse", post), {})
            self.assertEqual(store.lookup("parent", "child")["issue"], "issue")


if __name__ == "__main__":
    unittest.main()
