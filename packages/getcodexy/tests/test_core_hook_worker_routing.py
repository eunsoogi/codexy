"""Verify authenticated Worker model routing through the installed hook."""

from collections.abc import Generator, Mapping
from contextlib import contextmanager
import json
import os
import shutil
import subprocess
import unittest
from pathlib import Path
from typing import cast

# unittest discovery adds this directory to sys.path, so the helper is imported by its top-level name.
from core_hook_test_support import (  # pyright: ignore[reportImplicitRelativeImport]
    temporary_primary_checkout,
)


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins/codexy"
TIMING_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"
PARENT = "sender-thread"
WORKER = "recipient-thread"
TOOL = "codex_app__send_message_to_thread"
ROUTING = {
    "schema": "codexy.thread-delivery.v2",
    "authenticated": True,
    "direction": "root_to_child",
    "sender_thread_id": PARENT,
    "target_thread_id": WORKER,
    "target_model": "gpt-6-luna",
    "target_thinking": "max",
}
TOOL_INPUT = {
    "threadId": WORKER,
    "model": "gpt-6-luna",
    "thinking": "max",
    "prompt": "Continue the existing lane.",
}
SOL_ROUTING = {
    **ROUTING,
    "target_model": "gpt-6.1-sol",
    "target_thinking": "medium",
}
SOL_TOOL_INPUT = {
    **TOOL_INPUT,
    "model": "gpt-6.1-sol",
    "thinking": "medium",
}


class CoreHookWorkerRoutingTests(unittest.TestCase):
    def test_valid_worker_route_stays_admitted_when_timing_disabled_or_fails(
        self,
    ) -> None:
        with self._candidate() as plugin:
            target = plugin.parent / "timing.jsonl"
            quiet = self._run(plugin, "PreToolUse")
            self.assertEqual(
                (quiet.returncode, quiet.stdout, quiet.stderr), (0, b"", b"")
            )
            self.assertFalse(target.exists())

            measured = self._run(plugin, "PreToolUse", target)
            self.assertEqual((measured.returncode, measured.stderr), (0, b""))
            self.assertEqual(measured.stdout, quiet.stdout)
            self._assert_record(target, "PreToolUse", "allow")

            failure_target = plugin.parent / "timing-directory"
            failure_target.mkdir()
            failed_recording = self._run(plugin, "PreToolUse", failure_target)
            self.assertEqual(
                (failed_recording.returncode, failed_recording.stderr), (0, b"")
            )
            self.assertEqual(failed_recording.stdout, quiet.stdout)
            self.assertEqual(list(failure_target.iterdir()), [])

    def test_existing_sol_medium_worker_is_denied_and_timing_preserves_deny(
        self,
    ) -> None:
        with self._candidate() as plugin:
            for event in ("PermissionRequest", "PreToolUse"):
                target = plugin.parent / f"{event}.jsonl"
                baseline = self._run(
                    plugin,
                    event,
                    tool_input=SOL_TOOL_INPUT,
                    routing=SOL_ROUTING,
                )
                self._assert_denied(baseline, event, "MISMATCHED_ROUTING_METADATA")
                self.assertFalse(target.exists())

                measured = self._run(
                    plugin,
                    event,
                    target,
                    tool_input=SOL_TOOL_INPUT,
                    routing=SOL_ROUTING,
                )
                self._assert_denied(measured, event, "MISMATCHED_ROUTING_METADATA")
                self.assertEqual(measured.stdout, baseline.stdout)
                self._assert_record(target, event, "deny")

                failure_target = plugin.parent / f"{event}-directory"
                failure_target.mkdir()
                failed_recording = self._run(
                    plugin,
                    event,
                    failure_target,
                    tool_input=SOL_TOOL_INPUT,
                    routing=SOL_ROUTING,
                )
                self._assert_denied(
                    failed_recording, event, "MISMATCHED_ROUTING_METADATA"
                )
                self.assertEqual(failed_recording.stdout, baseline.stdout)
                self.assertEqual(list(failure_target.iterdir()), [])

    def test_invalid_recipient_stays_denied_and_records_deny(self) -> None:
        with self._candidate() as plugin:
            target = plugin.parent / "denial-timing.jsonl"
            invalid_input = {**TOOL_INPUT, "threadId": "other-recipient"}
            result = self._run(
                plugin,
                "PreToolUse",
                target,
                tool_input=invalid_input,
            )
            self._assert_denied(result, "PreToolUse", "WRONG_RECIPIENT")
            self._assert_record(target, "PreToolUse", "deny")

    def test_duplicate_routing_key_is_denied_without_extra_context(self) -> None:
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
            self._assert_denied(result, "PreToolUse", "CODEXY_THREAD_DELIVERY_ENVELOPE")
            self._assert_record(target, "PreToolUse", "deny")

    @contextmanager
    def _candidate(self) -> Generator[Path, None, None]:
        with temporary_primary_checkout() as primary:
            # Exercise a disposable installed copy with the current source, not personal plugin state.
            yield Path(shutil.copytree(PLUGIN, primary / "codexy"))

    def _run(
        self,
        plugin: Path,
        event: str,
        timing_target: Path | None = None,
        *,
        tool_input: Mapping[str, object] | None = None,
        routing: Mapping[str, object] | None = None,
    ) -> subprocess.CompletedProcess[bytes]:
        payload: dict[str, object] = {
            "hook_event_name": event,
            "tool_name": TOOL,
            "tool_input": TOOL_INPUT if tool_input is None else tool_input,
            "session_id": PARENT,
            "codexy_thread_delivery": ROUTING if routing is None else routing,
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
    ) -> subprocess.CompletedProcess[bytes]:
        environment = os.environ.copy()
        environment["PLUGIN_ROOT"] = str(plugin)
        _ = environment.pop(TIMING_ENV, None)
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

    def _assert_denied(
        self, result: subprocess.CompletedProcess[bytes], event: str, expected: str
    ) -> None:
        self.assertEqual((result.returncode, result.stderr), (0, b""))
        output = cast(dict[str, object], json.loads(result.stdout))
        specific = cast(dict[str, object], output["hookSpecificOutput"])
        self.assertEqual(specific["hookEventName"], event)
        self.assertNotIn("additionalContext", specific)
        if event == "PermissionRequest":
            decision = cast(dict[str, object], specific["decision"])
            self.assertEqual(decision["behavior"], "deny")
            reason = decision["message"]
        else:
            self.assertEqual(specific["permissionDecision"], "deny")
            reason = specific["permissionDecisionReason"]
        if not isinstance(reason, str):
            self.fail("denial reason must be text")
        self.assertIn(expected, reason)

    def _assert_record(self, path: Path, event: str, decision: str) -> None:
        records = [
            cast(dict[str, object], json.loads(line))
            for line in path.read_text().splitlines()
        ]
        self.assertEqual(len(records), 1)
        record = records[0]
        self.assertEqual(set(record), {"event", "concern", "elapsed", "decision"})
        self.assertEqual(record["event"], event)
        self.assertEqual(record["concern"], "thread-delivery")
        self.assertIsInstance(record["elapsed"], int)
        self.assertEqual(record["decision"], decision)
