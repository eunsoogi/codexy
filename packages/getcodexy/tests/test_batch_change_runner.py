from __future__ import annotations

import json
import os
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).parents[3]
INPUT_SCRIPTS = ROOT / "plugins/codexy/skills/engineering/scripts/batch_change_input"
RUNNER_SCRIPTS = ROOT / "plugins/codexy/skills/engineering/scripts/batch_change_runner"
sys.path.insert(0, str(INPUT_SCRIPTS))
from preview import preview  # noqa: E402

sys.path.insert(0, str(RUNNER_SCRIPTS))
from runner import run_batch  # noqa: E402


@unittest.skipUnless(os.name == "posix", "runner requires POSIX process groups")
class BatchChangeRunnerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "workspace"
        self.root.mkdir()
        self.results = Path(self.temporary.name) / "results"
        self.script = self.root / "commands.py"
        self.script.write_text(
            """from pathlib import Path
import subprocess, sys, time
m, a = sys.argv[1], sys.argv[2:]
if m == 'copy':
    s, d = map(Path, a[:2]); d.parent.mkdir(parents=True, exist_ok=True); d.write_text(s.read_text().upper())
elif m == 'bad':
    d = Path(a[1]); d.parent.mkdir(parents=True, exist_ok=True); d.write_text('bad')
elif m == 'fail':
    raise SystemExit(7)
elif m == 'validate':
    raise SystemExit(0 if Path(a[0]).read_text() != 'bad' else 9)
elif m == 'spam':
    print('SECRET_TOKEN ' * 10000, flush=True); print('SECRET_TOKEN ' * 10000, file=sys.stderr, flush=True)
elif m == 'mutate':
    d = Path(a[1]); d.parent.mkdir(parents=True, exist_ok=True); d.write_text('output'); Path(a[2]).write_text('changed')
elif m == 'spawn':
    child = subprocess.Popen([sys.executable, __file__, 'child']); Path(a[2]).write_text(str(child.pid)); time.sleep(30)
elif m == 'child':
    time.sleep(30)
else:
    raise SystemExit(2)
""",
            encoding="utf-8",
        )

    def _item(self, item_id, original, output, transform, validation, timeout=2):
        command = lambda argv, limit: {"argv": argv, "timeout_seconds": limit}
        return {
            "id": item_id,
            "original": original,
            "output": output,
            "transform": command(transform, timeout),
            "validations": [command(validation, timeout)],
        }

    def _preview(self, items: list[dict[str, object]]) -> dict[str, object]:
        return preview(self.root, {"items": items})

    def _run(self, document: dict[str, object], **kwargs: object) -> dict[str, object]:
        return run_batch(
            document, workspace_root=self.root, results_root=self.results, **kwargs
        )

    def _command(self, mode: str, *arguments: str) -> list[str]:
        return [sys.executable, str(self.script), mode, *arguments]

    @staticmethod
    def _alive(pid: int) -> bool:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        return True

    def _assert_dead(self, pid_file: Path) -> None:
        pid = int(pid_file.read_text(encoding="utf-8"))
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline and self._alive(pid):
            time.sleep(0.05)
        self.assertFalse(self._alive(pid), f"descendant {pid} survived cleanup")

    def test_isolates_failures_and_preserves_validated_artifacts(self) -> None:
        for name, content in (
            ("first.txt", "first\n"),
            ("second.txt", "second\n"),
            ("third.txt", "third\n"),
        ):
            (self.root / name).write_text(content, encoding="utf-8")
        document = self._preview(
            [
                self._item(
                    "failed-transform",
                    "first.txt",
                    "out/first.txt",
                    self._command("fail", "first.txt", "out/first.txt"),
                    self._command("validate", "out/first.txt"),
                ),
                self._item(
                    "success",
                    "second.txt",
                    "out/second.txt",
                    self._command("copy", "second.txt", "out/second.txt"),
                    self._command("validate", "out/second.txt"),
                ),
                self._item(
                    "failed-validation",
                    "third.txt",
                    "out/third.txt",
                    self._command("bad", "third.txt", "out/third.txt"),
                    self._command("validate", "out/third.txt"),
                ),
            ]
        )
        result = self._run(document)
        by_id = {item["id"]: item for item in result["items"]}
        self.assertEqual(result["status"], "completed")
        self.assertEqual(result["succeeded_count"], 1)
        self.assertEqual(result["failed_count"], 2)
        self.assertEqual(by_id["failed-transform"]["failure"]["reason"], "nonzero-exit")
        self.assertEqual(by_id["failed-validation"]["failure"]["phase"], "validation")
        output = Path(by_id["success"]["output"]["path"])
        self.assertEqual(output.read_text(encoding="utf-8"), "SECOND\n")
        self.assertEqual(by_id["success"]["output"]["validated"], True)
        self.assertEqual(
            [
                self.root.joinpath(name).read_text()
                for name in ("first.txt", "second.txt", "third.txt")
            ],
            ["first\n", "second\n", "third\n"],
        )

    def test_timeout_and_cancellation_kill_descendants(self) -> None:
        original = self.root / "input.txt"
        original.write_text("input", encoding="utf-8")
        timeout_pid = Path(self.temporary.name) / "timeout.pid"
        timeout_preview = self._preview(
            [
                self._item(
                    "timeout",
                    "input.txt",
                    "out.txt",
                    self._command("spawn", "input.txt", "out.txt", str(timeout_pid)),
                    self._command("validate", "out.txt"),
                    0.15,
                )
            ]
        )
        timeout_result = self._run(timeout_preview)
        self.assertEqual(timeout_result["items"][0]["failure"]["reason"], "timeout")
        self._assert_dead(timeout_pid)

        cancel_pid = Path(self.temporary.name) / "cancel.pid"
        cancel_preview = self._preview(
            [
                self._item(
                    "cancel",
                    "input.txt",
                    "cancel.txt",
                    self._command("spawn", "input.txt", "cancel.txt", str(cancel_pid)),
                    self._command("validate", "cancel.txt"),
                    30,
                )
            ]
        )
        cancellation = threading.Event()
        holder: dict[str, object] = {}
        worker = threading.Thread(
            target=lambda: holder.update(
                self._run(cancel_preview, cancellation_event=cancellation)
            )
        )
        worker.start()
        deadline = time.monotonic() + 2
        while not cancel_pid.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        cancellation.set()
        worker.join(3)
        self.assertFalse(worker.is_alive())
        self.assertEqual(holder["status"], "cancelled")
        self.assertEqual(holder["items"][0]["failure"]["reason"], "cancelled")
        self._assert_dead(cancel_pid)

    def test_excessive_output_is_bounded_and_not_returned(self) -> None:
        original = self.root / "input.txt"
        original.write_text("input", encoding="utf-8")
        document = self._preview(
            [
                self._item(
                    "output-limit",
                    "input.txt",
                    "out.txt",
                    self._command("spam"),
                    self._command("validate", "out.txt"),
                )
            ]
        )
        result = self._run(document, output_limit_bytes=128)
        self.assertEqual(result["items"][0]["failure"]["reason"], "output-limit")
        self.assertNotIn("SECRET_TOKEN", json.dumps(result))
        self.assertEqual(original.read_text(encoding="utf-8"), "input")

    def test_external_original_mutation_aborts_without_restoration(self) -> None:
        first = self.root / "first.txt"
        second = self.root / "second.txt"
        first.write_text("first", encoding="utf-8")
        second.write_text("second", encoding="utf-8")
        document = self._preview(
            [
                self._item(
                    "mutates-other",
                    "first.txt",
                    "out/first.txt",
                    self._command("mutate", "first.txt", "out/first.txt", str(second)),
                    self._command("validate", "out/first.txt"),
                ),
                self._item(
                    "must-not-run",
                    "second.txt",
                    "out/second.txt",
                    self._command("copy", "second.txt", "out/second.txt"),
                    self._command("validate", "out/second.txt"),
                ),
            ]
        )
        result = self._run(document)
        self.assertEqual(result["status"], "aborted")
        self.assertEqual(result["items"][0]["failure"]["reason"], "original-changed")
        self.assertEqual(result["items"][1]["status"], "not-run")
        self.assertEqual(second.read_text(encoding="utf-8"), "changed")


if __name__ == "__main__":
    unittest.main()
