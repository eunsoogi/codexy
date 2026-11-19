"""Verify supervision bootstrap and resumed delivery at the hook."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins/codexy"
from packages.getcodexy.tests.recipient_test_support import Dispatch, open_store


class RecipientResumeHookTests(unittest.TestCase):
    def test_active_owner_without_parent_bootstrap_is_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            os.chmod(directory, 0o700)
            store = open_store(directory)
            Dispatch(store)
            store.adopt(
                "parent", "request", "issue", "gpt-6.1-terra", "medium", "child"
            )
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "codex_app__send_message_to_thread",
                "session_id": "parent",
                "tool_input": {
                    "threadId": "child",
                    "model": "gpt-6.1-terra",
                    "thinking": "medium",
                },
            }
            result = subprocess.run(
                [
                    sys.executable,
                    str(PLUGIN / "hooks/codexy-thread-delivery.py"),
                    "--event",
                    "PreToolUse",
                ],
                input=json.dumps(payload),
                text=True,
                capture_output=True,
                env={**os.environ, "PLUGIN_DATA": directory},
                check=True,
            )
            self.assertIn("ASSIGNMENT_UNAVAILABLE", result.stdout)

    def test_released_owner_requires_resume_before_delivery(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            os.chmod(directory, 0o700)
            store = open_store(directory)
            Dispatch(store).bootstrap(
                "parent",
                [
                    {
                        "request": "request",
                        "issue": "issue",
                        "model": "gpt-6.1-terra",
                        "thinking": "medium",
                        "child": "child",
                    }
                ],
            )
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "codex_app__send_message_to_thread",
                "session_id": "parent",
                "tool_input": {
                    "threadId": "child",
                    "model": "gpt-6.1-terra",
                    "thinking": "medium",
                },
            }
            command = [
                sys.executable,
                str(PLUGIN / "hooks/codexy-thread-delivery.py"),
                "--event",
                "PreToolUse",
            ]
            env = {**os.environ, "PLUGIN_DATA": directory}
            store.release("parent", "request", confirmed=True)
            denied = subprocess.run(
                command,
                input=json.dumps(payload),
                text=True,
                capture_output=True,
                env=env,
                check=True,
            )
            self.assertIn("ASSIGNMENT_UNAVAILABLE", denied.stdout)
            self.assertEqual(
                store.reserve("parent", "request", "issue", "gpt-6.1-terra", "medium"),
                "resumed",
            )
            allowed = subprocess.run(
                command,
                input=json.dumps(payload),
                text=True,
                capture_output=True,
                env=env,
                check=True,
            )
            self.assertEqual(allowed.stdout, "")
