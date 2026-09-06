from contextlib import contextmanager
import importlib.util
import json
import os
import shutil
import stat
import subprocess
import tempfile
import threading
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
                self._assert_success(self._run_case(plugin, stem, tool, tool_input))
            self.assertFalse(target.exists())

    def test_enabled_real_launcher_preserves_allow_deny_and_fields(self) -> None:
        with self._candidate() as plugin:
            target = plugin.parent.resolve() / "timing with spaces.jsonl"
            for stem, tool, tool_input, concern in CASES:
                allowed = self._run_case(plugin, stem, tool, tool_input, target)
                self._assert_success(allowed)
                denied = self._run_case(plugin, stem, "wrong_tool", {}, target)
                self.assertEqual((denied.returncode, denied.stderr), (0, b""))
                denied_output = json.loads(denied.stdout)["hookSpecificOutput"]
                self.assertEqual(denied_output["hookEventName"], "PreToolUse")

            if os.name != "nt":
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
            result = self._run_case(
                plugin, CASES[0][0], CASES[0][1], CASES[0][2], target
            )
            self.assertEqual(result.stdout, b"")
            self.assertEqual(target.read_bytes(), b"seed")

            real = directory / "real.jsonl"
            link = directory / "link.jsonl"
            real.write_bytes(b"")
            real.chmod(0o600)
            link.symlink_to(real)
            denied = self._run_case(plugin, CASES[0][0], "wrong_tool", {}, link)
            self.assertEqual(denied.returncode, 0)
            self.assertTrue(link.is_symlink())
            self.assertEqual(real.read_bytes(), b"")

            full = directory / "full.jsonl"
            full.write_bytes(b"x" * MAX_BYTES)
            full.chmod(0o600)
            complete = self._run_case(
                plugin, CASES[0][0], CASES[0][1], CASES[0][2], full
            )
            self.assertEqual(complete.stdout, b"")
            self.assertEqual(full.stat().st_size, MAX_BYTES)

    @unittest.skipUnless(os.name != "nt", "POSIX descriptor and locking semantics")
    def test_ancestor_swap_and_serialized_writers_are_safe(self) -> None:
        with self._candidate() as plugin:
            directory = plugin.parent.resolve()
            parent = directory / "race-parent"
            held = directory / "held-parent"
            replacement = directory / "replacement-parent"
            parent.mkdir()
            replacement.mkdir()
            target = parent / "records.jsonl"
            timing = self._timing(plugin)
            original = timing._real_directory

            def swap_after_check(path: Path) -> bool:
                allowed = original(path)
                if allowed:
                    parent.rename(held)
                    parent.symlink_to(replacement, target_is_directory=True)
                return allowed

            setattr(timing, "_real_directory", swap_after_check)
            timing._append(target, b"race\n")
            self.assertFalse((replacement / target.name).exists())
            self.assertFalse((held / target.name).exists())
            setattr(timing, "_real_directory", original)
            target = plugin.parent.resolve() / "near-cap.jsonl"
            target.write_bytes(b"x" * (MAX_BYTES - 2))
            target.chmod(0o600)
            barrier = threading.Barrier(2)
            lock = threading.Lock()

            def acquire(_descriptor: int):
                barrier.wait()
                lock.acquire()
                return lock

            def release(_descriptor: int, held) -> None:
                held.release()

            setattr(timing, "_try_lock", acquire)
            setattr(timing, "_unlock", release)
            writers = [
                threading.Thread(target=timing._append, args=(target, b"xx"))
                for _ in range(2)
            ]
            for writer in writers:
                writer.start()
            for writer in writers:
                writer.join()
            self.assertEqual(target.stat().st_size, MAX_BYTES)

    def test_payload_path_and_enabled_overhead(self) -> None:
        with self._candidate() as plugin:
            directory = plugin.parent.resolve()
            target = directory / "trusted.jsonl"
            attacker = directory / "attacker.jsonl"
            tool_input = {
                **CASES[0][2],
                TIMING_ENV: str(attacker),
            }
            off_start = time.perf_counter_ns()
            off = self._run_case(plugin, CASES[0][0], CASES[0][1], tool_input)
            off_elapsed = time.perf_counter_ns() - off_start
            on_start = time.perf_counter_ns()
            on = self._run_case(plugin, CASES[0][0], CASES[0][1], tool_input, target)
            on_elapsed = time.perf_counter_ns() - on_start
            self._assert_success(off)
            self._assert_success(on)
            self.assertGreater(off_elapsed, 0)
            self.assertGreater(on_elapsed, 0)
            self.assertTrue(target.is_file())
            self.assertFalse(attacker.exists())

    @contextmanager
    def _candidate(self):
        with tempfile.TemporaryDirectory(prefix="codexy-hook-timing ") as temporary:
            plugin = Path(temporary) / "codexy"
            shutil.copytree(PLUGIN, plugin)
            yield plugin

    def _timing(self, plugin: Path):
        path = plugin / "hooks/codexy_policy/timing.py"
        spec = importlib.util.spec_from_file_location("candidate_timing", path)
        assert spec is not None and spec.loader is not None
        timing = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(timing)
        return timing

    def _assert_success(self, result) -> None:
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((result.stdout, result.stderr), (b"", b""))

    def _run_case(self, plugin, stem, tool, tool_input, timing_file=None):
        launcher = f"{stem}.{'cmd' if os.name == 'nt' else 'sh'}"
        return self._run(
            plugin,
            launcher,
            _payload("PreToolUse", tool, tool_input),
            timing_file,
        )

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
        command = (
            [str(path), "PreToolUse"]
            if os.name != "nt"
            else [
                os.environ.get("COMSPEC", "cmd.exe"),
                "/d",
                "/c",
                "call",
                str(path),
                "PreToolUse",
            ]
        )
        return subprocess.run(
            command, input=payload, capture_output=True, env=environment, check=False
        )


def _payload(event: str, tool: str, tool_input: dict[str, object]) -> bytes:
    return json.dumps(
        {"hook_event_name": event, "tool_name": tool, "tool_input": tool_input}
    ).encode()


if __name__ == "__main__":
    unittest.main()
