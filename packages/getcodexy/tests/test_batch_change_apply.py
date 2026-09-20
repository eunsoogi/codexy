"""Behavioral tests for selected batch-result application."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from threading import Event

ROOT = Path(__file__).parents[3]
SCRIPT_ROOT = ROOT / "plugins/codexy/skills/engineering/scripts"
sys.path.insert(0, str(SCRIPT_ROOT))

from batch_change_apply.errors import ApplyError  # noqa: E402
from batch_change_apply.workflow import apply_from_path  # noqa: E402


class BatchChangeApplyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "workspace"
        self.root.mkdir()
        self.results = Path(self.temporary.name) / "results"
        self.results.mkdir()

    def _state(self, path: Path) -> dict[str, object]:
        metadata = path.stat()
        return {
            "size": metadata.st_size,
            "mtime_ns": metadata.st_mtime_ns,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }

    def _item(self, item_id: str, *, successful: bool = True) -> dict[str, object]:
        original = self.root / f"original-{item_id}.txt"
        original.write_text(f"source {item_id}\n", encoding="utf-8")
        output_path = f"out/{item_id}.txt"
        artifact = self.results / f"{item_id}.txt"
        artifact.parent.mkdir(parents=True, exist_ok=True)
        artifact.write_text(f"result {item_id}\n", encoding="utf-8")
        item: dict[str, object] = {
            "id": item_id,
            "resolution": "rerun" if successful else "rerun",
            "invocations": 1,
            "status": "succeeded" if successful else "failed",
            "original": {"path": original.name, "state": {"path": original.name, **self._state(original)}},
            "output_path": output_path,
            "result": None,
        }
        if successful:
            item["result"] = {
                "status": "succeeded",
                "output": {
                    "path": str(artifact),
                    "relative_path": output_path,
                    "state": self._state(artifact),
                    "validated": True,
                },
                "execution": {
                    "transform": {"state": "exited", "exit_code": 0},
                    "validations": [{"state": "exited", "exit_code": 0}],
                },
            }
        return item

    def _result(self, items: list[dict[str, object]], batch_id: str = "apply-test") -> Path:
        payload = {
            "schema": "codexy.batch-change-resume.v1",
            "status": "completed",
            "workspace": {"path": str(self.root.resolve())},
            "results": {"root": str(self.results)},
            "batch_id": batch_id,
            "items": items,
        }
        path = self.root / "resume-result.json"
        path.write_text(json.dumps(payload), encoding="utf-8")
        return path

    def test_applies_only_selected_success_and_preserves_other_items(self) -> None:
        selected = self._item("selected")
        unselected = self._item("unselected")
        failed = self._item("failed", successful=False)
        result_path = self._result([selected, unselected, failed])

        result = apply_from_path(
            self.root,
            str(result_path),
            selected_ids=["selected"],
        )

        by_id = {item["id"]: item for item in result["items"]}
        self.assertEqual(result["status"], "completed")
        self.assertEqual(by_id["selected"]["resolution"], "applied")
        self.assertIn("+++ b/out/selected.txt", by_id["selected"]["diff"])
        self.assertTrue(by_id["selected"]["readback"])
        self.assertEqual(by_id["unselected"]["resolution"], "unselected")
        self.assertEqual(by_id["failed"]["resolution"], "unselected")
        self.assertEqual(
            (self.root / "out/selected.txt").read_text(encoding="utf-8"),
            "result selected\n",
        )
        self.assertFalse((self.root / "out/unselected.txt").exists())
        self.assertFalse((self.root / "out/failed.txt").exists())

    def test_changed_original_is_conflict_and_duplicate_is_completed(self) -> None:
        conflict_item = self._item("changed")
        conflict_result = self._result([conflict_item])
        (self.root / "original-changed.txt").write_text("user change\n", encoding="utf-8")

        conflict = apply_from_path(self.root, str(conflict_result), selected_ids=["changed"])

        self.assertEqual(conflict["status"], "conflict")
        self.assertEqual(conflict["items"][0]["reason"], "original-changed")
        self.assertFalse((self.root / "out/changed.txt").exists())

        duplicate_item = self._item("duplicate")
        duplicate_result = self._result([duplicate_item], batch_id="duplicate-test")
        first = apply_from_path(self.root, str(duplicate_result), selected_ids=["duplicate"])
        second = apply_from_path(self.root, str(duplicate_result), selected_ids=["duplicate"])

        self.assertEqual(first["items"][0]["resolution"], "applied")
        self.assertEqual(second["status"], "completed")
        self.assertEqual(second["items"][0]["resolution"], "completed")
        self.assertEqual(second["items"][0]["reason"], "already-applied")

    def test_interruption_before_replacement_is_resumable(self) -> None:
        item = self._item("interrupt")
        result_path = self._result([item])
        cancellation = Event()

        def interrupt_before_replace(path: Path) -> None:
            del path
            cancellation.set()

        interrupted = apply_from_path(
            self.root,
            str(result_path),
            selected_ids=["interrupt"],
            cancellation_event=cancellation,
            before_replace=interrupt_before_replace,
        )

        self.assertEqual(interrupted["status"], "interrupted")
        self.assertEqual(interrupted["items"][0]["resolution"], "incomplete")
        self.assertFalse((self.root / "out/interrupt.txt").exists())
        resumed = apply_from_path(self.root, str(result_path), selected_ids=["interrupt"])
        self.assertEqual(resumed["status"], "completed")
        self.assertEqual(resumed["items"][0]["resolution"], "applied")

    def test_rejects_duplicate_selection_without_mutation(self) -> None:
        item = self._item("duplicate-selection")
        result_path = self._result([item])
        with self.assertRaisesRegex(ApplyError, "unique"):
            apply_from_path(
                self.root,
                str(result_path),
                selected_ids=["duplicate-selection", "duplicate-selection"],
            )
        self.assertFalse((self.root / "out/duplicate-selection.txt").exists())


if __name__ == "__main__":
    unittest.main()
