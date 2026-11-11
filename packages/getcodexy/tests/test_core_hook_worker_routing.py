"""Validate installed thread-delivery admission without asserting host authorization."""

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
TOOL = "codex_app__send_message_to_thread"
LEGACY_METADATA = "codexy_thread_delivery"
ALLOWED_PAIRS = (
    ("gpt-6.1-sol", "low"),
    ("gpt-6-luna", "max"),
    ("gpt-6.1-terra", "medium"),
)
TOOL_INPUT = {
    "threadId": "known-worker",
    "model": "gpt-6-luna",
    "thinking": "max",
    "prompt": "Continue the existing lane.",
}
CROSSED_TOOL_INPUT = {
    **TOOL_INPUT,
    "model": "gpt-6.1-sol",
    "thinking": "medium",
}


class CoreHookWorkerRoutingTests(unittest.TestCase):
    def test_valid_worker_request_stays_admitted_when_timing_disabled_or_fails(
        self,
    ) -> None:
        with self._candidate() as plugin:
            target = plugin.parent / "timing.jsonl"
            quiet = self._run(plugin, "PreToolUse")
            self._assert_admitted(quiet)
            self.assertFalse(target.exists())

            measured = self._run(plugin, "PreToolUse", target)
            self._assert_admitted(measured)
            self._assert_record(target, "PreToolUse", "allow")

            failure_target = plugin.parent / "timing-directory"
            failure_target.mkdir()
            failed_recording = self._run(plugin, "PreToolUse", failure_target)
            self._assert_admitted(failed_recording)
            self.assertEqual(list(failure_target.iterdir()), [])

    def test_each_allowed_pair_is_admitted_for_both_hook_events(self) -> None:
        with self._candidate() as plugin:
            for event in ("PermissionRequest", "PreToolUse"):
                for model, thinking in ALLOWED_PAIRS:
                    with self.subTest(event=event, model=model, thinking=thinking):
                        tool_input = {
                            **TOOL_INPUT,
                            "model": model,
                            "thinking": thinking,
                        }
                        self._assert_admitted(
                            self._run(plugin, event, tool_input=tool_input)
                        )

    def test_crossed_pair_is_denied_and_timing_preserves_deny(self) -> None:
        with self._candidate() as plugin:
            for event in ("PermissionRequest", "PreToolUse"):
                target = plugin.parent / f"{event}.jsonl"
                baseline = self._run(plugin, event, tool_input=CROSSED_TOOL_INPUT)
                self._assert_denied(
                    baseline, event, "CODEXY_THREAD_DELIVERY_UNSUPPORTED_THINKING"
                )
                self.assertFalse(target.exists())

                measured = self._run(
                    plugin,
                    event,
                    target,
                    tool_input=CROSSED_TOOL_INPUT,
                )
                self._assert_denied(
                    measured, event, "CODEXY_THREAD_DELIVERY_UNSUPPORTED_THINKING"
                )
                self.assertEqual(measured.stdout, baseline.stdout)
                self._assert_record(target, event, "deny")

                failure_target = plugin.parent / f"{event}-directory"
                failure_target.mkdir()
                failed_recording = self._run(
                    plugin,
                    event,
                    failure_target,
                    tool_input=CROSSED_TOOL_INPUT,
                )
                self._assert_denied(
                    failed_recording,
                    event,
                    "CODEXY_THREAD_DELIVERY_UNSUPPORTED_THINKING",
                )
                self.assertEqual(failed_recording.stdout, baseline.stdout)
                self.assertEqual(list(failure_target.iterdir()), [])

    def test_empty_recipient_is_denied_but_another_nonempty_value_is_admitted(
        self,
    ) -> None:
        with self._candidate() as plugin:
            empty = {**TOOL_INPUT, "threadId": ""}
            denied = self._run(plugin, "PreToolUse", tool_input=empty)
            self._assert_denied(
                denied, "PreToolUse", "CODEXY_THREAD_DELIVERY_MISSING_RECIPIENT"
            )

            alternate = {**TOOL_INPUT, "threadId": "another-known-thread"}
            self._assert_admitted(self._run(plugin, "PreToolUse", tool_input=alternate))

    def test_duplicate_legacy_metadata_key_stays_an_envelope_error_for_both_events(
        self,
    ) -> None:
        with self._candidate() as plugin:
            for event in ("PermissionRequest", "PreToolUse"):
                target = plugin.parent / f"duplicate-{event}.jsonl"
                metadata = json.dumps({"authenticated": True}, separators=(",", ":"))
                payload = (
                    f'{{"hook_event_name":{json.dumps(event)},'
                    '"tool_name":"codex_app__send_message_to_thread",'
                    f'"tool_input":{json.dumps(TOOL_INPUT)},'
                    f'"{LEGACY_METADATA}":{metadata},'
                    f'"{LEGACY_METADATA}":{metadata}}}'
                ).encode()
                result = self._run_payload(plugin, event, payload, target)
                self._assert_denied(result, event, "CODEXY_THREAD_DELIVERY_ENVELOPE")
                self._assert_record(target, event, "deny")

    def test_malformed_json_stays_an_envelope_error_for_both_events(self) -> None:
        with self._candidate() as plugin:
            for event in ("PermissionRequest", "PreToolUse"):
                target = plugin.parent / f"malformed-{event}.jsonl"
                result = self._run_payload(plugin, event, b"{not-json", target)
                self._assert_denied(result, event, "CODEXY_THREAD_DELIVERY_ENVELOPE")
                self._assert_record(target, event, "deny")

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
    ) -> subprocess.CompletedProcess[bytes]:
        payload: dict[str, object] = {
            "hook_event_name": event,
            "tool_name": TOOL,
            "tool_input": TOOL_INPUT if tool_input is None else tool_input,
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

    def _assert_admitted(self, result: subprocess.CompletedProcess[bytes]) -> None:
        self.assertEqual(
            (result.returncode, result.stdout, result.stderr), (0, b"", b"")
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
