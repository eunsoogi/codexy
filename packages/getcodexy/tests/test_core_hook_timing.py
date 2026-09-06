from contextlib import contextmanager
import importlib.util
import json
import os
import shutil
import stat
import subprocess
import tempfile
import threading
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PLUGIN = ROOT / "plugins/codexy"
TIMING_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"
MAX_BYTES = 1024 * 1024
FIELDS = {"event", "concern", "elapsed", "decision"}
COMSPEC = os.environ.get("COMSPEC", "cmd.exe")
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
            attacker = plugin.parent.resolve() / "attacker.jsonl"
            for stem, tool, tool_input, concern in CASES:
                if concern == CASES[0][3]:
                    tool_input = {**tool_input, TIMING_ENV: str(attacker)}
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
                self.assertEqual(set(record), FIELDS)
                self.assertEqual(record["event"], "PreToolUse")
                self.assertIsInstance(record["elapsed"], int)
                self.assertGreaterEqual(record["elapsed"], 0)
                self.assertIn(record["decision"], {"allow", "deny"})
            self.assertFalse(attacker.exists())
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
            self._run_case(plugin, *CASES[0][:3], target)
            self.assertEqual(target.read_bytes(), b"seed")

            real = directory / "real.jsonl"
            link = directory / "link.jsonl"
            real.write_bytes(b"")
            real.chmod(0o600)
            link.symlink_to(real)
            denied = self._run_case(plugin, CASES[0][0], "wrong_tool", {}, link)
            self.assertEqual(
                (denied.returncode, link.is_symlink(), real.read_bytes()),
                (0, True, b""),
            )

            full = directory / "full.jsonl"
            full.write_bytes(b"x" * MAX_BYTES)
            full.chmod(0o600)
            self._run_case(plugin, *CASES[0][:3], full)
            self.assertEqual(full.stat().st_size, MAX_BYTES)
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
            self._run_writers(timing, target)
            self.assertEqual(target.stat().st_size, MAX_BYTES)

    @unittest.skipUnless(os.name == "nt", "Windows native filesystem semantics")
    def test_windows_junction_acl_and_native_cap_are_failure_neutral(self) -> None:
        with self._candidate() as plugin:
            directory = plugin.parent.resolve()
            timing = self._timing(plugin)
            parent = directory / "race-parent"
            replacement = directory / "replacement-parent"
            parent.mkdir()
            replacement.mkdir()
            self._make_junction(parent, replacement)
            target = parent / "records.jsonl"
            timing._append(target, b"race\n")
            self.assertFalse((replacement / target.name).exists())
            parent.unlink()

            target = directory / "private.jsonl"
            timing._append(target, b"seed\n")
            subprocess.run(
                ["icacls", str(target), "/grant", "*S-1-1-0:F"],
                check=True,
                capture_output=True,
            )
            timing._append(target, b"blocked\n")
            self.assertEqual(target.read_bytes(), b"seed\n")

            target = directory / "near-cap.jsonl"
            timing._append(target, b"")
            with target.open("r+b") as stream:
                stream.truncate(MAX_BYTES - 2)
            self._run_writers(timing, target)
            self.assertEqual(target.stat().st_size, MAX_BYTES)

    def _make_junction(self, link: Path, target: Path) -> None:
        subprocess.run(
            [COMSPEC, "/d", "/c", "call", "mklink", "/J", str(link), str(target)],
            check=True,
            capture_output=True,
        )

    def _run_writers(self, timing, target: Path) -> None:
        barrier = threading.Barrier(2)

        def append() -> None:
            barrier.wait()
            timing._append(target, b"xx")

        writers = [threading.Thread(target=append) for _ in range(2)]
        for writer in writers:
            writer.start()
        for writer in writers:
            writer.join()

    @contextmanager
    def _candidate(self):
        with tempfile.TemporaryDirectory(prefix="codexy-hook-timing ") as temporary:
            yield shutil.copytree(PLUGIN, Path(temporary) / "codexy")

    def _timing(self, plugin: Path):
        path = plugin / "hooks/codexy_policy/timing.py"
        spec = importlib.util.spec_from_file_location("candidate_timing", path)
        timing = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(timing)
        return timing

    def _assert_success(self, result) -> None:
        self.assertEqual(
            (result.returncode, result.stdout, result.stderr), (0, b"", b"")
        )

    def _run_case(self, plugin, stem, tool, tool_input, timing_file=None):
        environment = os.environ.copy()
        environment["PLUGIN_ROOT"] = str(plugin)
        environment.pop(TIMING_ENV, None)
        if timing_file is not None:
            environment[TIMING_ENV] = str(timing_file)
        path = plugin / "hooks" / f"{stem}.{'cmd' if os.name == 'nt' else 'sh'}"
        command = [str(path), "PreToolUse"]
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
            input=json.dumps(
                {
                    "hook_event_name": "PreToolUse",
                    "tool_name": tool,
                    "tool_input": tool_input,
                }
            ).encode(),
            capture_output=True,
            env=environment,
            check=False,
        )


if __name__ == "__main__":
    unittest.main()
