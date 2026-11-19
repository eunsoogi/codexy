"""Exercise real delivery hook input against a reconciled supervision store."""

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


class RecipientDeliveryHookTests(unittest.TestCase):
    def test_uninitialized_and_corrupt_store_are_not_unrelated(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            os.chmod(directory, 0o700)
            open_store(directory)
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "codex_app__send_message_to_thread",
                "session_id": "sender",
                "tool_input": {
                    "threadId": "recipient",
                    "model": "gpt-6.2-sol",
                    "thinking": "low",
                },
            }
            command = [
                sys.executable,
                str(PLUGIN / "hooks/codexy-thread-delivery.py"),
                "--event",
                "PreToolUse",
            ]
            env = {**os.environ, "PLUGIN_DATA": directory}
            for state in ("uninitialized", "corrupt"):
                with self.subTest(state=state):
                    if state == "corrupt":
                        (
                            Path(directory) / "supervision/reservations.sqlite3"
                        ).write_bytes(b"not a database")
                    result = subprocess.run(
                        command,
                        input=json.dumps(payload),
                        text=True,
                        capture_output=True,
                        env=env,
                        check=True,
                    )
                    self.assertIn("ASSIGNMENT_UNAVAILABLE", result.stdout)

    def test_delivery_directions_registered_and_unregistered(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            os.chmod(directory, 0o700)
            store = open_store(directory)
            owners = [
                {
                    "request": "request",
                    "issue": "issue",
                    "model": "gpt-6.1-terra",
                    "thinking": "medium",
                    "child": "child",
                },
                {
                    "request": "hard-request",
                    "issue": "hard-issue",
                    "model": "gpt-6.2-sol",
                    "thinking": "medium",
                    "child": "hard-child",
                },
            ]
            Dispatch(store).bootstrap("parent", owners)
            Dispatch(store).bootstrap("unrelated", [])
            for event in ("PreToolUse", "PermissionRequest"):
                for sender, recipient, model, thinking, denied in (
                    ("child", "parent", None, None, True),
                    ("child", "parent", "gpt-6.1-terra", "medium", True),
                    ("child", "parent", "gpt-6.2-sol", "medium", True),
                    ("child", "parent", "gpt-6.2-sol", "low", False),
                    ("parent", "child", "gpt-6.1-terra", "medium", False),
                    ("parent", "hard-child", "gpt-6.2-sol", "medium", False),
                    ("ordinary", "another", "gpt-6.2-sol", "medium", False),
                    ("parent", "unrelated", "gpt-6.2-sol", "medium", True),
                    ("child", "another", "gpt-6.2-sol", "low", True),
                    ("parent", "unknown", "gpt-6.2-sol", "low", True),
                    ("unknown", "parent", "gpt-6.2-sol", "low", True),
                ):
                    with self.subTest(
                        event=event, sender=sender, recipient=recipient, model=model
                    ):
                        payload = {
                            "hook_event_name": event,
                            "tool_name": "codex_app__send_message_to_thread",
                            "session_id": sender,
                            "tool_input": {
                                "threadId": recipient,
                                "model": model,
                                "thinking": thinking,
                            },
                        }
                        env = {
                            **os.environ,
                            "PLUGIN_ROOT": str(PLUGIN),
                            "PLUGIN_DATA": directory,
                        }
                        result = subprocess.run(
                            [
                                sys.executable,
                                str(PLUGIN / "hooks/codexy-thread-delivery.py"),
                                "--event",
                                event,
                            ],
                            input=json.dumps(payload),
                            text=True,
                            capture_output=True,
                            env=env,
                            check=True,
                        )
                        self.assertEqual(bool(result.stdout), denied, result.stdout)
                        if denied and sender == "child" and recipient == "parent":
                            self.assertIn("gpt-6.2-sol/low", result.stdout)

    def test_nonroot_and_unavailable_state_fail_closed(self) -> None:
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
                "tool_name": "mcp__codex_app__send_message_to_thread",
                "session_id": "parent",
                "agent_id": "native-subagent",
                "tool_input": {
                    "threadId": "child",
                    "model": "gpt-6.1-terra",
                    "thinking": "medium",
                },
            }
            env = {**os.environ, "PLUGIN_DATA": directory}
            command = [
                sys.executable,
                str(PLUGIN / "hooks/codexy-thread-delivery.py"),
                "--event",
                "PreToolUse",
            ]
            result = subprocess.run(
                command,
                input=json.dumps(payload),
                text=True,
                capture_output=True,
                env=env,
                check=True,
            )
            self.assertIn("ASSIGNMENT_UNAVAILABLE", result.stdout)
            payload.pop("agent_id")
            env["PLUGIN_DATA"] = ""
            result = subprocess.run(
                command,
                input=json.dumps(payload),
                text=True,
                capture_output=True,
                env=env,
                check=True,
            )
            self.assertIn("ASSIGNMENT_UNAVAILABLE", result.stdout)


if __name__ == "__main__":
    unittest.main()
