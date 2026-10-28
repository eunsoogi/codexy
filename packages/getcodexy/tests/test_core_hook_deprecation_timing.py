"""Keep the legacy Worker advisory separate from denial and timing outcomes."""

from contextlib import contextmanager
import json
import os
import shutil
import subprocess
import unittest
from pathlib import Path

from core_hook_test_support import temporary_primary_checkout


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins/codexy"
TIMING_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"
PARENT = "sender-thread"
WORKER = "recipient-thread"
TOOL = "codex_app__send_message_to_thread"
WARNING = (
    "CODEXY_THREAD_DELIVERY_DEPRECATED_WORKER_ROUTE: The gpt-6.1-sol/medium Worker "
    "route is deprecated in codexy. Create new hard Workers with gpt-6.1-terra/medium "
    "and default Workers with gpt-6-luna/max. This delivery preserves the existing "
    "recipient assignment. Do not change its model or retry this delivery solely "
    "because of this warning."
)
ROUTING = {
    "schema": "codexy.thread-delivery.v2",
    "authenticated": True,
    "direction": "root_to_child",
    "sender_thread_id": PARENT,
    "target_thread_id": WORKER,
    "target_model": "gpt-6.1-sol",
    "target_thinking": "medium",
}
TOOL_INPUT = {
    "threadId": WORKER,
    "model": "gpt-6.1-sol",
    "thinking": "medium",
    "prompt": "Continue the existing lane.",
}


class CoreHookDeprecationTimingTests(unittest.TestCase):
    def test_warning_stays_admitted_when_timing_is_disabled_or_fails(self) -> None:
        with self._candidate() as plugin:
            target = plugin.parent / "timing.jsonl"
            quiet = self._run(plugin, "PreToolUse")
            self._assert_warning(quiet)
            self.assertFalse(target.exists())

            measured = self._run(plugin, "PreToolUse", target)
            self._assert_warning(measured)
            self.assertEqual(measured.stdout, quiet.stdout)
            self._assert_record(target, "PreToolUse", "allow")

            failure_target = plugin.parent / "timing-directory"
            failure_target.mkdir()
            failed_recording = self._run(plugin, "PreToolUse", failure_target)
            self._assert_warning(failed_recording)
            self.assertEqual(failed_recording.stdout, quiet.stdout)
            self.assertEqual(list(failure_target.iterdir()), [])

    def test_permission_request_stays_quiet_and_records_allow(self) -> None:
        with self._candidate() as plugin:
            target = plugin.parent / "permission-timing.jsonl"
            result = self._run(plugin, "PermissionRequest", target)
            self.assertEqual(
                (result.returncode, result.stdout, result.stderr), (0, b"", b"")
            )
            self._assert_record(target, "PermissionRequest", "allow")

    def test_invalid_recipient_stays_denied_and_records_deny(self) -> None:
        with self._candidate() as plugin:
            target = plugin.parent / "denial-timing.jsonl"
            invalid_input = {**TOOL_INPUT, "threadId": "other-recipient"}
            result = self._run(plugin, "PreToolUse", target, tool_input=invalid_input)
            self.assertEqual((result.returncode, result.stderr), (0, b""))
            specific = json.loads(result.stdout)["hookSpecificOutput"]
            self.assertEqual(specific["hookEventName"], "PreToolUse")
            self.assertEqual(specific["permissionDecision"], "deny")
            self.assertNotIn("additionalContext", specific)
            self._assert_record(target, "PreToolUse", "deny")

    def test_duplicate_routing_key_is_denied_without_a_warning(self) -> None:
        with self._candidate() as plugin:
            target = plugin.parent / "duplicate-timing.jsonl"
            metadata = json.dumps(ROUTING, separators=(",", ":"))
            payload = (
                '{"hook_event_name":"PreToolUse",'
                '"tool_name":"codex_app__send_message_to_thread",'
                f'"tool_input":{json.dumps(TOOL_INPUT)},'
                f'"session_id":{json.dumps(PARENT)},'
                f'"codexy_thread_delivery":{metadata},'
                f'"codexy_thread_delivery":{metadata}}}'
            ).encode()
            result = self._run_payload(plugin, "PreToolUse", payload, target)
            self.assertEqual((result.returncode, result.stderr), (0, b""))
            specific = json.loads(result.stdout)["hookSpecificOutput"]
            self.assertEqual(specific["hookEventName"], "PreToolUse")
            self.assertEqual(specific["permissionDecision"], "deny")
            self.assertNotIn("additionalContext", specific)
            self._assert_record(target, "PreToolUse", "deny")

    @contextmanager
    def _candidate(self):
        with temporary_primary_checkout() as primary:
            # Exercise a disposable installed copy with the current source, not personal plugin state.
            yield shutil.copytree(PLUGIN, primary / "codexy")

    def _run(
        self,
        plugin: Path,
        event: str,
        timing_target: Path | None = None,
        *,
        tool_input: dict[str, object] | None = None,
    ):
        payload = {
            "hook_event_name": event,
            "tool_name": TOOL,
            "tool_input": TOOL_INPUT if tool_input is None else tool_input,
            "session_id": PARENT,
            "codexy_thread_delivery": ROUTING,
        }
        return self._run_payload(
            plugin, event, json.dumps(payload).encode(), timing_target
        )

    def _run_payload(
        self,
        plugin: Path,
        event: str,
        payload: bytes,
        timing_target: Path | None,
    ):
        environment = os.environ.copy()
        environment["PLUGIN_ROOT"] = str(plugin)
        environment.pop(TIMING_ENV, None)
        if timing_target is not None:
            environment[TIMING_ENV] = str(timing_target)
        launcher = (
            plugin
            / "hooks"
            / (f"codexy-thread-delivery.{'cmd' if os.name == 'nt' else 'sh'}")
        )
        command = [str(launcher), event]
        if os.name == "nt":
            command = [
                os.environ.get("COMSPEC", "cmd.exe"),
                "/d",
                "/c",
                "call",
                *command,
            ]
        return subprocess.run(
            command,
            input=payload,
            capture_output=True,
            env=environment,
            check=False,
        )

    def _assert_warning(self, result) -> None:
        self.assertEqual((result.returncode, result.stderr), (0, b""))
        self.assertEqual(
            json.loads(result.stdout),
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": WARNING,
                }
            },
        )

    def _assert_record(self, path: Path, event: str, decision: str) -> None:
        records = [json.loads(line) for line in path.read_text().splitlines()]
        self.assertEqual(len(records), 1)
        record = records[0]
        self.assertEqual(set(record), {"event", "concern", "elapsed", "decision"})
        self.assertEqual(record["event"], event)
        self.assertEqual(record["concern"], "thread-delivery")
        self.assertIsInstance(record["elapsed"], int)
        self.assertEqual(record["decision"], decision)
