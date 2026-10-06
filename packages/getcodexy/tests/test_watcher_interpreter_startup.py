"""Test Watcher version gating and native-before-Rosetta interpreter fallback."""

from __future__ import annotations

import json
import os
import runpy
import shlex
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from typing import cast


PLUGIN = Path(__file__).resolve().parents[3] / "plugins/codexy"
HOOKS = PLUGIN / "hooks"


class WatcherInterpreterStartupTests(unittest.TestCase):
    def test_version_gate_uses_the_event_helper_before_the_hook_body(self):
        source = (HOOKS / "codexy_watcher_interrupt.py").read_text(encoding="utf-8")
        events = (HOOKS / "codexy_watcher_interrupt_events.py").read_text(
            encoding="utf-8"
        )
        self.assertIn("UNSUPPORTED_INTERPRETER_EXIT = 125", events)
        self.assertIn("raise SystemExit(UNSUPPORTED_INTERPRETER_EXIT)", source)
        gate = source.index("if sys.version_info < (3, 10):")
        self.assertLess(gate, source.index("def main"))
        self.assertIn("read(1024 * 1024 + 1)", source)

    def test_runtime_lookup_error_is_recorded_without_changing_hook_failure(self):
        helper = runpy.run_path(
            str(HOOKS / "codexy_watcher_interrupt_runtime.py")
        )
        records = []

        class Trace:
            def record(self, event, **fields):
                records.append({"event": event, **fields})

        def failed_lookup():
            raise OSError("private runtime path")

        with self.assertRaisesRegex(OSError, "private runtime path"):
            helper["_invoke"](
                "--hook-pretool",
                {"sessionId": "private-session", "parentToken": "private-token"},
                Trace(),
                failed_lookup,
            )

        self.assertEqual(
            records,
            [
                {
                    "event": "runtime_result",
                    "runtimeAvailable": False,
                    "failureClass": "runtime_lookup_error",
                }
            ],
        )
        encoded = json.dumps(records)
        self.assertNotIn("private runtime path", encoded)
        self.assertNotIn("private-session", encoded)
        self.assertNotIn("private-token", encoded)

    @unittest.skipUnless(os.name != "nt", "POSIX launcher coverage")
    def test_native_macos_python_binds_before_rosetta_fallback(self):
        with tempfile.TemporaryDirectory() as temporary:
            temp = Path(temporary)
            root = temp / "codexy"
            _ = shutil.copytree(PLUGIN, root)
            runtime_dir = temp / "runtime"
            _ = runtime_dir.mkdir()
            runtime = runtime_dir / "codexy-mcp-watcher-darwin-arm64.bin"
            _ = runtime.write_text(
                textwrap.dedent(
                    f"""\
                    #!{sys.executable}
                    import json, sys
                    payload = json.load(sys.stdin)
                    if sys.argv[1] != '--hook-pretool' or not payload.get('tool_name', '').endswith('__watcher_wait'):
                        raise SystemExit(2)
                    print(json.dumps({{'requestBinding': 'native-binding'}}))
                    """
                ),
                encoding="utf-8",
            )
            runtime.chmod(0o755)

            interpreter = temp / "interpreter_shim.py"
            _ = interpreter.write_text(
                textwrap.dedent(
                    """\
                    import os, platform, runpy, sys
                    from pathlib import Path
                    architecture = sys.argv[1]
                    arguments = sys.argv[2:]
                    if arguments[:2] != ['-I', '-B']:
                        raise SystemExit(2)
                    target = arguments[2]
                    Path(os.environ['PLUGIN_ROOT'], 'selected-interpreter.txt').write_text(architecture)
                    platform.system = lambda: 'Darwin'
                    platform.machine = lambda: architecture
                    sys.argv = [target, *arguments[3:]]
                    runpy.run_path(target, run_name='__main__')
                    """
                ),
                encoding="utf-8",
            )

            # Mirror Homebrew, Rosetta, and older system Python candidates.
            native = self._interpreter(temp / "homebrew-python3", interpreter, "arm64")
            rosetta = self._interpreter(temp / "rosetta-python3", interpreter, "x86_64")
            too_old = temp / "system-python3"
            _ = too_old.write_text("#!/bin/sh\nexit 125\n", encoding="utf-8")
            too_old.chmod(0o755)

            runtime_script = root / "hooks/codexy-hook-runtime.sh"
            source = runtime_script.read_text(encoding="utf-8")
            candidate_line = next(
                line
                for line in source.splitlines()
                if line.startswith("for candidate in ")
            )
            replacements = {
                "/opt/homebrew/bin/python3": native,
                "/usr/local/bin/python3": rosetta,
                "/usr/bin/python3": too_old,
            }
            candidates = (
                candidate_line.removeprefix("for candidate in ")
                .removesuffix("; do")
                .split()
            )
            self.assertTrue(candidates)
            self.assertTrue(all(candidate in replacements for candidate in candidates))
            fixture_line = (
                "for candidate in "
                + " ".join(
                    shlex.quote(str(replacements[candidate]))
                    for candidate in candidates
                )
                + "; do"
            )
            _ = runtime_script.write_text(
                source.replace(candidate_line, fixture_line), encoding="utf-8"
            )

            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "mcp__codex_app__watcher_wait",
                "tool_input": {"sessionId": "session-under-test"},
            }
            environment = {
                "HOME": str(temp),
                "USER": "watcher-test",
                "PLUGIN_ROOT": str(root),
                "CODEXY_RUNTIME_DIR": str(runtime_dir),
            }
            result = subprocess.run(
                [str(root / "hooks/codexy-watcher-interrupt.sh"), "PreToolUse"],
                input=json.dumps(payload).encode(),
                capture_output=True,
                env=environment,
                check=False,
            )

            self.assertEqual((result.returncode, result.stderr), (0, b""))
            selected = (root / "selected-interpreter.txt").read_text(encoding="utf-8")
            self.assertEqual(selected, "arm64")
            parsed: object = cast(object, json.loads(result.stdout))
            output = _string_keyed_dict(parsed)
            hook_output = _string_keyed_dict(output["hookSpecificOutput"])
            updated = _string_keyed_dict(hook_output["updatedInput"])
            self.assertEqual(updated["requestBinding"], "native-binding")
            self.assertEqual(updated["sessionId"], "session-under-test")

    @staticmethod
    def _interpreter(path: Path, shim: Path, architecture: str) -> Path:
        _ = path.write_text(
            textwrap.dedent(
                f"""\
                #!/bin/sh
                exec {shlex.quote(sys.executable)} -I -B {shlex.quote(str(shim))} {shlex.quote(architecture)} "$@"
                """
            ),
            encoding="utf-8",
        )
        path.chmod(0o755)
        return path


def _string_keyed_dict(value: object) -> dict[str, object]:
    if not isinstance(value, dict):
        raise AssertionError("expected a JSON object with string keys")
    untyped = cast(dict[object, object], value)
    if any(not isinstance(key, str) for key in untyped):
        raise AssertionError("expected a JSON object with string keys")
    return cast(dict[str, object], value)
