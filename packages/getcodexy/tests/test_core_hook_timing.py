from __future__ import annotations

import json
import os
import shutil
import stat
import subprocess
import tempfile
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins/codexy"
TIMING_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"
MAX_BYTES = 1024 * 1024
CASES = (
    (
        "codexy-thread-delivery",
        "mcp__codex_app__send_message_to_thread",
        {"threadId": "parent", "model": "gpt-6-astra", "thinking": "medium"},
        "thread-delivery",
    ),
    (
        "codexy-child-thread-creation",
        "mcp__codex_app__create_thread",
        {"model": "gpt-5.6-luna", "thinking": "max"},
        "child-thread-creation",
    ),
    (
        "codexy-subagent-ownership",
        "multi_agent_v1__spawn_agent",
        {"agent_type": "codexy-cartographer", "message": "Map files only."},
        "subagent-ownership",
    ),
)


class CoreHookTimingTests(unittest.TestCase):
    def test_default_off_skips_timing_import_and_file_io(self) -> None:
        with self._candidate() as plugin:
            timing = plugin / "hooks/codexy_policy/timing.py"
            timing.unlink()
            target = plugin.parent.resolve() / "timing.jsonl"
            for stem, tool, tool_input, _ in CASES:
                result = self._run(
                    plugin,
                    self._launcher(stem),
                    _payload("PreToolUse", tool, tool_input),
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, b"")
                self.assertEqual(result.stderr, b"")
            self.assertFalse(target.exists())

    def test_enabled_real_launcher_preserves_allow_deny_and_allowed_fields(
        self,
    ) -> None:
        with self._candidate() as plugin:
            target = plugin.parent.resolve() / "timing with spaces.jsonl"
            for stem, tool, tool_input, concern in CASES:
                allowed = self._run(
                    plugin,
                    self._launcher(stem),
                    _payload("PreToolUse", tool, tool_input),
                    target,
                )
                self.assertEqual(allowed.returncode, 0, allowed.stderr)
                self.assertEqual(allowed.stdout, b"")
                self.assertEqual(allowed.stderr, b"")
                denied = self._run(
                    plugin,
                    self._launcher(stem),
                    _payload("PreToolUse", "wrong_tool", {}),
                    target,
                )
                self.assertEqual(denied.returncode, 0, denied.stderr)
                self.assertEqual(denied.stderr, b"")
                self.assertEqual(
                    json.loads(denied.stdout)["hookSpecificOutput"]["hookEventName"],
                    "PreToolUse",
                )

            self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o600)
            records = [json.loads(line) for line in target.read_text().splitlines()]
            self.assertEqual(len(records), 6)
            for record in records:
                self.assertEqual(
                    set(record), {"event", "concern", "elapsed", "decision"}
                )
                self.assertEqual(record["event"], "PreToolUse")
                self.assertIsInstance(record["elapsed"], int)
                self.assertGreaterEqual(record["elapsed"], 0)
                self.assertIn(record["decision"], {"allow", "deny"})
            self.assertEqual(
                [record["concern"] for record in records],
                [case[3] for case in CASES for _ in (0, 1)],
            )

    @unittest.skipUnless(os.name != "nt", "POSIX permission and symlink semantics")
    def test_unwritable_symlink_and_full_targets_are_failure_neutral(self) -> None:
        with self._candidate() as plugin:
            directory = plugin.parent.resolve()
            target = directory / "unwritable.jsonl"
            target.write_bytes(b"seed")
            target.chmod(0o400)
            result = self._run(
                plugin,
                self._launcher("codexy-thread-delivery"),
                _payload(
                    "PreToolUse",
                    "mcp__codex_app__send_message_to_thread",
                    CASES[0][2],
                ),
                target,
            )
            self.assertEqual(result.stdout, b"")
            self.assertEqual(target.read_bytes(), b"seed")

            real = directory / "real.jsonl"
            link = directory / "link.jsonl"
            real.write_bytes(b"")
            real.chmod(0o600)
            link.symlink_to(real)
            denied = self._run(
                plugin,
                self._launcher("codexy-thread-delivery"),
                _payload("PreToolUse", "wrong_tool", {}),
                link,
            )
            self.assertEqual(denied.returncode, 0)
            self.assertTrue(link.is_symlink())
            self.assertEqual(real.read_bytes(), b"")

            full = directory / "full.jsonl"
            full.write_bytes(b"x" * MAX_BYTES)
            full.chmod(0o600)
            complete = self._run(
                plugin,
                self._launcher("codexy-thread-delivery"),
                _payload(
                    "PreToolUse",
                    "mcp__codex_app__send_message_to_thread",
                    CASES[0][2],
                ),
                full,
            )
            self.assertEqual(complete.stdout, b"")
            self.assertEqual(full.stat().st_size, MAX_BYTES)

    def test_payload_cannot_select_target_and_enabled_overhead_is_measured(
        self,
    ) -> None:
        with self._candidate() as plugin:
            directory = plugin.parent.resolve()
            target = directory / "trusted.jsonl"
            attacker = directory / "attacker.jsonl"
            tool_input = {
                **CASES[0][2],
                TIMING_ENV: str(attacker),
            }
            off_start = time.perf_counter_ns()
            off = self._run(
                plugin,
                self._launcher("codexy-thread-delivery"),
                _payload("PreToolUse", CASES[0][1], tool_input),
            )
            off_elapsed = time.perf_counter_ns() - off_start
            on_start = time.perf_counter_ns()
            on = self._run(
                plugin,
                self._launcher("codexy-thread-delivery"),
                _payload("PreToolUse", CASES[0][1], tool_input),
                target,
            )
            on_elapsed = time.perf_counter_ns() - on_start
            self.assertEqual(off.stdout, b"")
            self.assertEqual(on.stdout, b"")
            self.assertEqual(off.stderr, b"")
            self.assertEqual(on.stderr, b"")
            self.assertGreater(off_elapsed, 0)
            self.assertGreater(on_elapsed, 0)
            self.assertTrue(target.is_file())
            self.assertFalse(attacker.exists())

    def _candidate(self):
        temporary = tempfile.TemporaryDirectory(prefix="codexy-hook-timing ")
        plugin = Path(temporary.name) / "codexy"
        shutil.copytree(PLUGIN, plugin)
        return _TemporaryPlugin(temporary, plugin)

    def _launcher(self, stem: str) -> str:
        return f"{stem}.{'cmd' if os.name == 'nt' else 'sh'}"

    def _run(
        self,
        plugin: Path,
        launcher: str,
        payload: bytes,
        timing_file: Path | None = None,
    ) -> subprocess.CompletedProcess[bytes]:
        environment = os.environ.copy()
        environment["PLUGIN_ROOT"] = str(plugin)
        environment.pop(TIMING_ENV, None)
        if timing_file is not None:
            environment[TIMING_ENV] = str(timing_file)
        path = plugin / "hooks" / launcher
        command = [str(path), "PreToolUse"]
        if os.name == "nt":
            command = [
                os.environ.get("COMSPEC", "cmd.exe"),
                "/d",
                "/s",
                "/c",
                f'"{path}" PreToolUse',
            ]
        return subprocess.run(
            command,
            input=payload,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            check=False,
        )


class _TemporaryPlugin:
    def __init__(self, temporary: tempfile.TemporaryDirectory, plugin: Path):
        self.temporary = temporary
        self.plugin = plugin

    def __enter__(self) -> Path:
        return self.plugin

    def __exit__(self, *exc_info) -> None:
        self.temporary.cleanup()


def _payload(event: str, tool: str, tool_input: dict[str, object]) -> bytes:
    return json.dumps(
        {"hook_event_name": event, "tool_name": tool, "tool_input": tool_input}
    ).encode()


if __name__ == "__main__":
    unittest.main()
