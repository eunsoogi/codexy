"""Documented cleanup inputs never establish archival or eligibility."""

import sys
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[3] / "plugins" / "codexy" / "hooks"
sys.path.insert(0, str(HOOKS))

from codexy_policy.cleanup_progress import continuation, observation  # noqa: E402


class CleanupObservationTests(unittest.TestCase):
    def test_archive_return_requires_exact_input_and_nonerror_envelope(self):
        payload = {
            "hook_event_name": "PostToolUse",
            "tool_name": "mcp__codex_app__set_thread_archived",
            "tool_input": {"threadId": "child-123", "archived": True},
            "tool_response": {"content": []},
        }
        self.assertEqual(observation(payload), ("archive_returned", "child-123"))
        payload["tool_response"] = {"content": [], "isError": True}
        self.assertIsNone(observation(payload))
        payload["tool_response"] = "unrecognized result"
        self.assertIsNone(observation(payload))

    def test_readback_attempt_does_not_claim_confirmation(self):
        payload = {
            "hook_event_name": "PostToolUse",
            "tool_name": "mcp__codex_app__list_archived_threads",
        }
        self.assertEqual(observation(payload), ("list_attempt", None))
        message = continuation("child-123")["reason"]
        self.assertIn("unconfirmed", message)
        self.assertIn("Do not infer acceptance", message)

    def test_stop_readback_is_bounded_to_turn_and_suppressed_after_attempt(self):
        with tempfile.TemporaryDirectory() as directory:
            environment = {**os.environ, "PLUGIN_DATA": directory}

            def invoke(event, **fields):
                payload = {
                    "hook_event_name": event,
                    "session_id": "s",
                    "turn_id": "t",
                    **fields,
                }
                result = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        str(HOOKS / "codexy_cleanup_progress.py"),
                        "--event",
                        event,
                    ],
                    input=json.dumps(payload),
                    text=True,
                    capture_output=True,
                    env=environment,
                    check=True,
                )
                return json.loads(result.stdout)

            self.assertEqual(invoke("Stop", stop_hook_active=False), {})
            invoke(
                "PostToolUse",
                tool_name="mcp__codex_app__set_thread_archived",
                tool_input={"threadId": "child-123", "archived": True},
                tool_response={"content": []},
            )
            self.assertEqual(
                invoke("Stop", stop_hook_active=False)["decision"], "block"
            )
            self.assertEqual(invoke("Stop", stop_hook_active=True), {})
            invoke("PostToolUse", tool_name="mcp__codex_app__list_archived_threads")
            self.assertEqual(invoke("Stop", stop_hook_active=False), {})
            # A duplicate archive event cannot reinstate a completed list attempt.
            invoke(
                "PostToolUse",
                tool_name="mcp__codex_app__set_thread_archived",
                tool_input={"threadId": "child-123", "archived": True},
                tool_response={"content": []},
            )
            self.assertEqual(invoke("Stop", stop_hook_active=False), {})

    def test_interruption_and_other_turn_do_not_continue(self):
        with tempfile.TemporaryDirectory() as directory:
            environment = {**os.environ, "PLUGIN_DATA": directory}

            def invoke(event, turn="one", **fields):
                payload = {
                    "hook_event_name": event,
                    "session_id": "session",
                    "turn_id": turn,
                    **fields,
                }
                result = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        str(HOOKS / "codexy_cleanup_progress.py"),
                        "--event",
                        event,
                    ],
                    input=json.dumps(payload),
                    text=True,
                    capture_output=True,
                    env=environment,
                    check=True,
                )
                return json.loads(result.stdout)

            invoke(
                "PostToolUse",
                tool_name="mcp__codex_app__set_thread_archived",
                tool_input={"threadId": "child-123", "archived": True},
                tool_response={"content": []},
            )
            self.assertEqual(invoke("Stop", turn="two", stop_hook_active=False), {})
            invoke("Interrupt")
            self.assertEqual(invoke("Stop", stop_hook_active=False), {})

    def test_stale_marker_never_requests_continuation(self):
        with tempfile.TemporaryDirectory() as directory:
            environment = {**os.environ, "PLUGIN_DATA": directory}

            def invoke(event, **fields):
                payload = {
                    "hook_event_name": event,
                    "session_id": "session",
                    "turn_id": "turn",
                    **fields,
                }
                result = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        str(HOOKS / "codexy_cleanup_progress.py"),
                        "--event",
                        event,
                    ],
                    input=json.dumps(payload),
                    text=True,
                    capture_output=True,
                    env=environment,
                    check=True,
                )
                return json.loads(result.stdout)

            invoke(
                "PostToolUse",
                tool_name="mcp__codex_app__set_thread_archived",
                tool_input={"threadId": "child-123", "archived": True},
                tool_response={"content": []},
            )
            marker = next((Path(directory) / "cleanup-progress").glob("*.archive"))
            os.utime(marker, (1, 1))
            self.assertEqual(invoke("Stop", stop_hook_active=False), {})

    @unittest.skipIf(os.name == "nt", "POSIX launcher")
    def test_packaged_launcher_passes_host_data_to_cleanup_concern(self):
        with tempfile.TemporaryDirectory() as directory:
            environment = {
                **os.environ,
                "PLUGIN_ROOT": str(HOOKS.parent),
                "PLUGIN_DATA": directory,
            }

            def invoke(event, **fields):
                payload = {
                    "hook_event_name": event,
                    "session_id": "session",
                    "turn_id": "turn",
                    **fields,
                }
                result = subprocess.run(
                    [str(HOOKS / "codexy-cleanup-progress.sh"), event],
                    input=json.dumps(payload),
                    text=True,
                    capture_output=True,
                    env=environment,
                    check=True,
                )
                return json.loads(result.stdout)

            invoke(
                "PostToolUse",
                tool_name="mcp__codex_app__set_thread_archived",
                tool_input={"threadId": "child-123", "archived": True},
                tool_response={"content": []},
            )
            self.assertEqual(
                invoke("Stop", stop_hook_active=False)["decision"], "block"
            )

    def test_unrecognized_result_reports_unknown_without_continuation(self):
        with tempfile.TemporaryDirectory() as directory:
            payload = {
                "hook_event_name": "PostToolUse",
                "session_id": "session",
                "turn_id": "turn",
                "tool_name": "mcp__codex_app__set_thread_archived",
                "tool_input": {"threadId": "child-123", "archived": True},
                "tool_response": "unknown format",
            }
            result = subprocess.run(
                [
                    sys.executable,
                    "-I",
                    str(HOOKS / "codexy_cleanup_progress.py"),
                    "--event",
                    "PostToolUse",
                ],
                input=json.dumps(payload),
                text=True,
                capture_output=True,
                env={**os.environ, "PLUGIN_DATA": directory},
                check=True,
            )
            self.assertIn("unknown", json.loads(result.stdout)["systemMessage"])


if __name__ == "__main__":
    unittest.main()
