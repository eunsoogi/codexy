from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[6]
INPUT_SCRIPTS = ROOT / "plugins/codexy/skills/engineering/scripts/batch_change_input"
RUNNER_SCRIPTS = ROOT / "plugins/codexy/skills/engineering/scripts/batch_change_runner"
RUNNER = RUNNER_SCRIPTS / "batch_change_runner.py"
COMMAND_SOURCE = "from pathlib import Path\nimport subprocess,sys,time\nm,a=sys.argv[1],sys.argv[2:]\nif m=='copy': s,d=map(Path,a[:2]); d.parent.mkdir(parents=True,exist_ok=True); d.write_text(s.read_text().upper())\nelif m=='bad': d=Path(a[1]); d.parent.mkdir(parents=True,exist_ok=True); d.write_text('bad')\nelif m=='fail': raise SystemExit(7)\nelif m=='delete': Path(a[0]).unlink(); raise SystemExit(7)\nelif m=='validate': raise SystemExit(0 if Path(a[0]).read_text()!='bad' else 9)\nelif m=='spam': print('SECRET_TOKEN '*10000,flush=True); print('SECRET_TOKEN '*10000,file=sys.stderr,flush=True)\nelif m=='mutate': d=Path(a[1]); d.parent.mkdir(parents=True,exist_ok=True); d.write_text('output'); Path(a[2]).write_text('changed')\nelif m=='spawn': child=subprocess.Popen([sys.executable,__file__,'child']); Path(a[2]).write_text(str(child.pid)); time.sleep(30)\nelif m=='symlink': Path(a[0]).symlink_to(Path(a[1]),target_is_directory=True)\nelif m=='child': time.sleep(30)\nelse: raise SystemExit(2)\n"

sys.path.insert(0, str(INPUT_SCRIPTS))
from preview import preview  # noqa: E402

sys.path.insert(0, str(RUNNER_SCRIPTS))
from runner import run_batch  # noqa: E402


class BatchChangeRunnerCase(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "workspace"
        self.root.mkdir()
        self.results = Path(self.temporary.name) / "results"
        self.script = self.root / "commands.py"
        self.script.write_text(COMMAND_SOURCE, encoding="utf-8")

    def _command(self, *arguments: str) -> list[str]:
        return [sys.executable, str(self.script), *arguments]

    def _item(
        self,
        item_id: str,
        original: str,
        output: str,
        mode: str,
        *arguments: object,
        timeout: int | float = 2,
    ) -> dict[str, object]:
        if arguments and isinstance(arguments[-1], (int, float)):
            timeout, arguments = arguments[-1], arguments[:-1]
        command = lambda argv: {
            "argv": self._command(*argv),
            "timeout_seconds": timeout,
        }
        return {
            "id": item_id,
            "original": original,
            "output": output,
            "transform": command((mode, *arguments)),
            "validations": [command(("validate", output))],
        }

    def _preview(self, *items: dict[str, object]) -> dict[str, object]:
        return preview(self.root, {"items": list(items)})

    def _run_document(
        self, document: dict[str, object], **kwargs: object
    ) -> dict[str, object]:
        return run_batch(
            document, workspace_root=self.root, results_root=self.results, **kwargs
        )

    def _run(self, *items: dict[str, object], **kwargs: object) -> dict[str, object]:
        return self._run_document(self._preview(*items), **kwargs)

    def _run_specs(
        self, *specs: tuple[object, ...], **kwargs: object
    ) -> dict[str, object]:
        return self._run(*(self._item(*spec) for spec in specs), **kwargs)

    def _manifest(self, name: str, *items: dict[str, object]) -> Path:
        path = self.root / name
        path.write_text(json.dumps({"items": list(items)}), encoding="utf-8")
        return path

    def _manifest_specs(self, name: str, *specs: tuple[object, ...]) -> Path:
        return self._manifest(name, *(self._item(*spec) for spec in specs))

    def _cli(self, manifest: Path, *arguments: str) -> list[str]:
        return [
            sys.executable,
            str(RUNNER),
            "--workspace-root",
            str(self.root),
            "--input",
            str(manifest),
            "--results-root",
            str(self.results),
            *arguments,
        ]

    def _cli_result(
        self, manifest: Path, *arguments: str
    ) -> tuple[subprocess.CompletedProcess[str], dict[str, object]]:
        completed = subprocess.run(
            self._cli(manifest, *arguments), text=True, capture_output=True, check=False
        )
        return completed, json.loads(completed.stdout)

    def _write(self, *names: str) -> None:
        for name in names:
            (self.root / name).write_text(name, encoding="utf-8")

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

    def _reason(self, result: dict[str, object], reason: str, index: int = 0) -> None:
        self.assertEqual(result["items"][index]["failure"]["reason"], reason)

    def _aborted(self, result: dict[str, object], reason: str) -> None:
        self.assertEqual(result["status"], "aborted")
        self._reason(result, reason)
