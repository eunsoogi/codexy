"""Exercise the actual caller-visible output through isolated subprocesses."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from packages.getcodexy.tests.supervision_test_support import dispatch, storage

HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"


class SupervisionDiagnosticsTests(unittest.TestCase):
    def test_boundary_reasons_keep_private_details_and_existing_ownership(self):
        with tempfile.TemporaryDirectory(prefix="private-diagnostics-") as root:
            tool_input = {"model": "m", "thinking": "low", "prompt": "private input"}
            event = {
                "hook_event_name": "PreToolUse",
                "tool_name": "mcp__codex_app__create_thread",
                "session_id": "private-parent",
                "tool_use_id": "private-call",
                "tool_input": tool_input,
            }

            def call(script, arguments, data, plugin_root=root):
                outcome = subprocess.run(
                    [sys.executable, "-I", "-B", str(HOOKS / script), *arguments],
                    input=json.dumps(data),
                    text=True,
                    capture_output=True,
                    env={**os.environ, "PLUGIN_DATA": plugin_root},
                )
                self.assertFalse(outcome.stderr)
                for secret in (
                    root,
                    event["session_id"],
                    event["tool_use_id"],
                    data.get("prompt", "private input"),
                ):
                    self.assertNotIn(secret, outcome.stdout)
                return outcome.returncode, json.loads(outcome.stdout)

            def hook(plugin_root=root):
                code, result = call(
                    "codexy_supervision_capacity.py",
                    ["--event", "PreToolUse"],
                    event,
                    plugin_root,
                )
                self.assertEqual(code, 0)
                return result

            def prep(issue, data, plugin_root=root):
                return call(
                    "codexy_supervision_prepare.py",
                    [
                        "--plugin-data",
                        plugin_root,
                        "--parent",
                        event["session_id"],
                        "--issue",
                        issue,
                        "--limit",
                        "1",
                    ],
                    data,
                    plugin_root,
                )

            def deny(result, reason, action):
                output = result["hookSpecificOutput"]
                self.assertEqual(output["permissionDecision"], "deny")
                self.assertTrue(
                    output["permissionDecisionReason"].startswith(reason + ":")
                )
                self.assertIn(action, output["permissionDecisionReason"])

            invalid_root = str(Path(root) / "nonexistent")
            deny(
                hook(invalid_root),
                "CODEXY_SUPERVISION_STORE_UNAVAILABLE",
                "private reservation store",
            )
            code, result = prep("new", tool_input, invalid_root)
            self.assertEqual(
                (code, result["reason"]), (1, "CODEXY_SUPERVISION_STORE_UNAVAILABLE")
            )
            if os.name != "nt":
                unsafe = Path(root) / "shared"
                unsafe.mkdir()
                unsafe.chmod(0o755)
                deny(
                    hook(str(unsafe)),
                    "CODEXY_SUPERVISION_STORE_UNAVAILABLE",
                    "private reservation store",
                )
                self.assertFalse((unsafe / "supervision").exists())
            deny(
                hook(),
                "CODEXY_SUPERVISION_PARENT_UNINITIALIZED",
                "bootstrap --confirmed-complete",
            )
            code, result = prep("new", tool_input)
            self.assertEqual(
                (code, result["reason"]), (1, "CODEXY_SUPERVISION_PARENT_UNINITIALIZED")
            )

            store = storage.open_store(root)
            dispatch.Dispatch(store).bootstrap(event["session_id"], [])
            deny(
                hook(),
                "CODEXY_SUPERVISION_CAPACITY_UNPREPARED",
                "prepare the exact current",
            )
            self.assertEqual(prep("first", tool_input)[1]["result"], "reserved")
            self.assertEqual(hook(), {})
            other = {**tool_input, "prompt": "other private input"}
            code, result = prep("first", other)
            self.assertEqual(
                (code, result["reason"]), (1, "CODEXY_SUPERVISION_PREPARE_REJECTED")
            )
            self.assertIn("reuse an existing owner", result["nextAction"])
            code, result = prep("second", other)
            self.assertEqual(
                (code, result["result"], result["reason"]),
                (0, "queued", "CODEXY_SUPERVISION_SLOT_LIMIT"),
            )
            self.assertIn("continue independent parent work", result["nextAction"])


if __name__ == "__main__":
    unittest.main()
