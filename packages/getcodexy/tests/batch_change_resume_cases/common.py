"""Shared fixtures and real-process helpers for resume scenarios."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[4]
RESUME_SCRIPTS = ROOT / "plugins/codexy/skills/engineering/scripts/batch_change_resume"
INPUT_SCRIPTS = ROOT / "plugins/codexy/skills/engineering/scripts/batch_change_input"
CLI = RESUME_SCRIPTS / "batch_change_resume.py"
sys.path.insert(0, str(INPUT_SCRIPTS))
sys.path.insert(0, str(RESUME_SCRIPTS))

from preview import preview  # noqa: E402
from resume import ResumeError, resume_batch  # noqa: E402


COMMAND_SOURCE = """
from pathlib import Path
import sys
import time

mode, args = sys.argv[1], sys.argv[2:]

def record(path, value):
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open("a", encoding="utf-8") as output:
        output.write(value + "\\n")

if mode in {"copy", "slow-copy"}:
    source, destination, log = args[:3]
    record(log, "transform")
    if mode == "slow-copy":
        time.sleep(float(args[3]))
    target = Path(destination)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(Path(source).read_text(encoding="utf-8").upper(), encoding="utf-8")
elif mode in {"validate", "slow-validate"}:
    output, log = args[:2]
    record(log, "validation")
    if mode == "slow-validate":
        time.sleep(float(args[2]))
    raise SystemExit(0 if Path(output).read_text(encoding="utf-8").isupper() else 9)
else:
    raise SystemExit(2)
"""


class BatchChangeResumeCase(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "workspace"
        self.root.mkdir()
        self.original = self.root / "input.txt"
        self.original.write_text("hello\n", encoding="utf-8")
        self.command = self.root / "commands.py"
        self.command.write_text(COMMAND_SOURCE, encoding="utf-8")
        self.transform_log = Path(self.temporary.name) / "transform.log"
        self.validation_log = Path(self.temporary.name) / "validation.log"

    def _argv(self, *values: object) -> list[str]:
        return [sys.executable, str(self.command), *(str(value) for value in values)]

    def _item(
        self,
        *,
        transform: str = "copy",
        validation: str = "validate",
        transform_timeout: int | float = 5,
        validation_timeout: int | float = 5,
        transform_delay: float = 0.0,
        validation_delay: float = 0.0,
        item_id: str = "item",
    ) -> dict[str, object]:
        transform_args: list[object] = [
            transform,
            "input.txt",
            "output.txt",
            self.transform_log,
        ]
        if transform == "slow-copy":
            transform_args.append(transform_delay)
        validation_args: list[object] = [validation, "output.txt", self.validation_log]
        if validation == "slow-validate":
            validation_args.append(validation_delay)
        return {
            "id": item_id,
            "original": "input.txt",
            "output": "output.txt",
            "transform": {
                "argv": self._argv(*transform_args),
                "timeout_seconds": transform_timeout,
            },
            "validations": [
                {
                    "argv": self._argv(*validation_args),
                    "timeout_seconds": validation_timeout,
                }
            ],
        }

    def _preview(self, item: dict[str, object] | None = None) -> dict[str, object]:
        return preview(self.root, {"items": [item or self._item()]})

    def _run(
        self, item: dict[str, object] | None = None, **kwargs: object
    ) -> dict[str, object]:
        return resume_batch(self._preview(item), workspace_root=self.root, **kwargs)

    def _manifest(self, item: dict[str, object] | None = None) -> Path:
        path = self.root / "batch.json"
        path.write_text(json.dumps({"items": [item or self._item()]}), encoding="utf-8")
        return path

    def _cli_args(
        self,
        manifest: Path,
        *,
        state_root: Path | None = None,
        results_root: Path | None = None,
    ) -> list[str]:
        return [
            sys.executable,
            str(CLI),
            "--workspace-root",
            str(self.root),
            "--input",
            str(manifest),
            "--state-root",
            str(state_root or self.root / ".codexy-batch-state"),
            "--results-root",
            str(results_root or self.root / ".codexy-batch-results"),
        ]

    def _cli_result(
        self,
        manifest: Path,
        *,
        process: subprocess.Popen[str] | None = None,
    ) -> tuple[subprocess.CompletedProcess[str] | tuple[int, str], dict[str, object]]:
        if process is not None:
            stdout, stderr = process.communicate(timeout=10)
            self.assertEqual(stderr, "", stderr)
            return (process.returncode, stdout), json.loads(stdout)
        completed = subprocess.run(
            self._cli_args(manifest),
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.stderr, "", completed.stderr)
        return completed, json.loads(completed.stdout)

    def _state_file(self) -> Path:
        state_files = list((self.root / ".codexy-batch-state").glob("*.json"))
        self.assertEqual(len(state_files), 1)
        return state_files[0]

    def _wait_for_state(self, expected_status: str = "in-progress") -> Path:
        deadline = time.monotonic() + 5
        state_file = self.root / ".codexy-batch-state" / "placeholder.json"
        while time.monotonic() < deadline:
            candidates = list((self.root / ".codexy-batch-state").glob("*.json"))
            if candidates:
                state_file = candidates[0]
                try:
                    state = json.loads(state_file.read_text(encoding="utf-8"))
                except (OSError, json.JSONDecodeError):
                    state = {}
                if (
                    state.get("items", {}).get("item", {}).get("status")
                    == expected_status
                ):
                    return state_file
            time.sleep(0.02)
        self.fail(f"state did not reach {expected_status}: {state_file}")

    def _log_count(self, path: Path, value: str) -> int:
        if not path.exists():
            return 0
        return path.read_text(encoding="utf-8").splitlines().count(value)
