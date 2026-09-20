"""Installed outside-checkout proof for the complete batch-change flow."""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

from packages.getcodexy.tests.batch_change_installed_support import (
    install_core_component,
    run_boundary_interruption,
)

ROOT = Path(__file__).parents[3]


@unittest.skipUnless(os.name == "posix", "installed batch flow requires POSIX signals")
class BatchChangeInstalledTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "synthetic-workspace"
        self.root.mkdir()
        self.installed = Path(self.temporary.name) / "installed"
        self.installed_plugin = install_core_component(ROOT, self.installed)
        installed_scripts = self.installed_plugin / "skills/engineering/scripts"
        self.resume_cli = (
            installed_scripts / "batch_change_resume/batch_change_resume.py"
        )
        self.apply_cli = installed_scripts / "batch_change.py"
        self.command = self.root / "trusted-transform.py"
        self.command.write_text(
            """import sys, time
from pathlib import Path
mode, source, output, *rest = sys.argv[1:]
if mode == 'fail': raise SystemExit(9)
if mode == 'slow': time.sleep(float(rest[0]))
if mode == 'validate': raise SystemExit(0 if Path(source).read_text().isupper() else 8)
target = Path(output)
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(Path(source).read_text().upper())
""",
            encoding="utf-8",
        )
        self.environment = os.environ.copy()
        self.environment.pop("PYTHONPATH", None)
        self.environment.pop("PYTHONHOME", None)
        self.environment["PYTHONDONTWRITEBYTECODE"] = "1"

    def _manifest(self) -> Path:
        items: list[dict[str, object]] = []
        for index in range(20):
            item_id = f"item-{index:02d}"
            source = self.root / f"source-{index:02d}.txt"
            source.write_text(
                ("large " if index == 15 else "")
                + f"source {item_id}\n"
                + ("x" * 8_000_000 if index == 15 else ""),
                encoding="utf-8",
            )
            mode = "fail" if index == 3 else "slow" if index == 10 else "copy"
            transform = [
                sys.executable,
                str(self.command),
                mode,
                source.name,
                f"out/{item_id}.txt",
            ]
            if mode == "slow":
                transform.append("2")
            items.append(
                {
                    "id": item_id,
                    "original": source.name,
                    "output": f"out/{item_id}.txt",
                    "transform": {"argv": transform, "timeout_seconds": 10},
                    "validations": [
                        {
                            "argv": [
                                sys.executable,
                                str(self.command),
                                "validate",
                                f"out/{item_id}.txt",
                                "ignored",
                            ],
                            "timeout_seconds": 10,
                        }
                    ],
                }
            )
        manifest = self.root / "batch.json"
        manifest.write_text(json.dumps({"items": items}), encoding="utf-8")
        return manifest

    def _args(self, script: Path, manifest: Path) -> list[str]:
        return [
            sys.executable,
            str(script),
            "--workspace-root",
            str(self.root),
            "--input",
            str(manifest),
            "--state-root",
            str(self.root / ".codexy-batch-state"),
            "--results-root",
            str(self.root / ".codexy-batch-results"),
        ]

    def _start_resume(self, manifest: Path) -> subprocess.Popen[str]:
        return subprocess.Popen(
            self._args(self.resume_cli, manifest),
            cwd=self.root,
            env=self.environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def _run_resume(self, manifest: Path) -> dict[str, object]:
        completed = subprocess.run(
            self._args(self.resume_cli, manifest),
            cwd=self.root,
            env=self.environment,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(completed.stderr, "")
        return json.loads(completed.stdout)

    def _wait_for_resume_state(self) -> Path:
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            for path in (self.root / ".codexy-batch-state").glob("*.json"):
                try:
                    value = json.loads(path.read_text(encoding="utf-8"))
                except (OSError, json.JSONDecodeError):
                    continue
                if any(
                    entry.get("status") == "in-progress"
                    for entry in value.get("items", {}).values()
                ):
                    return path
            time.sleep(0.01)
        self.fail("resume state did not reach in-progress")

    def _run_apply(self, result_path: Path, *selected: str) -> dict[str, object]:
        completed = subprocess.run(
            [
                sys.executable,
                str(self.apply_cli),
                "--workspace-root",
                str(self.root),
                "--results",
                str(result_path),
                "--select",
                *selected,
            ],
            cwd=self.root,
            env=self.environment,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr + completed.stdout)
        self.assertEqual(completed.stderr, "")
        return json.loads(completed.stdout)

    def test_installed_flow_is_conflict_safe_and_resumable(self) -> None:
        manifest = self._manifest()
        first = self._start_resume(manifest)
        self._wait_for_resume_state()
        first.send_signal(signal.SIGINT)
        stdout, stderr = first.communicate(timeout=15)
        self.assertEqual((first.returncode, stderr), (0, ""))
        self.assertEqual(json.loads(stdout)["status"], "interrupted")
        resume_result = self._run_resume(manifest)
        self.assertEqual(resume_result["status"], "completed")
        self.assertEqual(resume_result["items"][3]["status"], "failed")
        result_path = self.root / "resume-result.json"
        result_path.write_text(json.dumps(resume_result), encoding="utf-8")
        (self.root / "source-04.txt").write_text("user changed\n", encoding="utf-8")

        applied = self._run_apply(result_path, "item-00", "item-04", "item-10")
        applied_by_id = {item["id"]: item for item in applied["items"]}
        self.assertEqual(applied["status"], "conflict")
        self.assertEqual(applied_by_id["item-00"]["resolution"], "applied")
        self.assertEqual(applied_by_id["item-04"]["reason"], "original-changed")
        self.assertTrue(applied_by_id["item-10"]["readback"])
        self.assertTrue((self.root / "out/item-00.txt").is_file())
        self.assertFalse((self.root / "out/item-04.txt").exists())
        self.assertTrue((self.root / "out/item-10.txt").is_file())

        duplicate = self._run_apply(result_path, "item-00", "item-04", "item-10")
        duplicate_by_id = {item["id"]: item for item in duplicate["items"]}
        self.assertEqual(duplicate_by_id["item-00"]["resolution"], "completed")
        self.assertEqual(duplicate_by_id["item-00"]["reason"], "already-applied")
        self.assertEqual(duplicate_by_id["item-10"]["resolution"], "completed")
        self.assertEqual(duplicate_by_id["item-04"]["reason"], "original-changed")
        edited_output = self.root / "out/item-00.txt"
        edited_output.write_text("user edit\n", encoding="utf-8")
        repeated_conflict = self._run_apply(result_path, "item-00")
        self.assertEqual(repeated_conflict["status"], "conflict")
        self.assertEqual(repeated_conflict["items"][0]["reason"], "destination-changed")
        self.assertEqual(edited_output.read_text(encoding="utf-8"), "user edit\n")

        interrupted_result = run_boundary_interruption(
            sys.executable,
            self.installed_plugin / "skills/engineering/scripts",
            self.root,
            Path(self.temporary.name),
            result_path,
            "item-01",
            self.environment,
        )
        self.assertEqual(interrupted_result["status"], "interrupted")
        self.assertEqual(
            interrupted_result["items"][1]["reason"], "interrupted before replacement"
        )
        self.assertFalse((self.root / "out/item-01.txt").exists())
        resumed_apply = self._run_apply(result_path, "item-01")
        self.assertEqual(resumed_apply["status"], "completed")
        self.assertIn(resumed_apply["items"][1]["resolution"], {"applied", "completed"})
        self.assertTrue((self.root / "out/item-01.txt").is_file())
        for provenance in (applied["provenance"], interrupted_result["provenance"]):
            entrypoint = Path(provenance["entrypoint"]).resolve()
            module = Path(provenance["module"]).resolve()
            self.assertTrue(entrypoint.is_relative_to(self.installed.resolve()))
            self.assertTrue(module.is_relative_to(self.installed.resolve()))
            self.assertNotIn(str(ROOT), str(entrypoint))


if __name__ == "__main__":
    unittest.main()
