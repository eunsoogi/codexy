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
            "original": {
                "path": original.name,
                "state": {"path": original.name, **self._state(original)},
            },
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

    def _result(
        self, items: list[dict[str, object]], batch_id: str = "apply-test"
    ) -> Path:
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

    def _cancel_before_replace(self, cancellation: Event):
        def interrupt(path: Path) -> None:
            del path
            cancellation.set()

        return interrupt

    def _apply(self, result_path: Path, item_id: str, **kwargs: object):
        return apply_from_path(
            self.root, str(result_path), selected_ids=[item_id], **kwargs
        )

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
        (self.root / "original-changed.txt").write_text(
            "user change\n", encoding="utf-8"
        )

        conflict = self._apply(conflict_result, "changed")

        self.assertEqual(conflict["status"], "conflict")
        self.assertEqual(conflict["items"][0]["reason"], "original-changed")
        self.assertFalse((self.root / "out/changed.txt").exists())

        duplicate_item = self._item("duplicate")
        duplicate_result = self._result([duplicate_item], batch_id="duplicate-test")
        first = self._apply(duplicate_result, "duplicate")
        second = self._apply(duplicate_result, "duplicate")

        self.assertEqual(first["items"][0]["resolution"], "applied")
        self.assertEqual(second["status"], "completed")
        self.assertEqual(second["items"][0]["resolution"], "completed")
        self.assertEqual(second["items"][0]["reason"], "already-applied")
        cancellation = Event()
        cancellation.set()
        interrupted = self._apply(
            duplicate_result, "duplicate", cancellation_event=cancellation
        )
        self.assertEqual(interrupted["status"], "interrupted")
        (self.root / "out/duplicate.txt").write_text("user edit\n", encoding="utf-8")
        edited = self._apply(duplicate_result, "duplicate")
        self.assertEqual(edited["status"], "conflict")
        self.assertEqual(edited["items"][0]["reason"], "destination-changed")
        repeated = self._apply(duplicate_result, "duplicate")
        self.assertEqual(repeated["status"], "conflict")
        self.assertEqual(repeated["items"][0]["reason"], "destination-changed")
        self.assertEqual(
            (self.root / "out/duplicate.txt").read_text(encoding="utf-8"),
            "user edit\n",
        )

    def test_interruption_reports_selected_and_unselected_remaining_items(self) -> None:
        items = [self._item(item_id) for item_id in ("first", "second", "third")]
        result_path = self._result(items, batch_id="interrupted-report")
        cancellation = Event()

        result = apply_from_path(
            self.root,
            str(result_path),
            selected_ids=["first", "third"],
            cancellation_event=cancellation,
            before_replace=self._cancel_before_replace(cancellation),
        )

        by_id = {item["id"]: item for item in result["items"]}
        self.assertEqual(result["status"], "interrupted")
        self.assertEqual(result["item_count"], 3)
        self.assertEqual(by_id["first"]["resolution"], "incomplete")
        self.assertEqual(by_id["second"]["resolution"], "unselected")
        self.assertEqual(by_id["third"]["resolution"], "incomplete")

    def test_rejects_state_root_escape_before_creating_it(self) -> None:
        item = self._item("state-escape")
        result_path = self._result([item], batch_id="state-escape")
        outside = self.root.parent / "outside-state"

        with self.assertRaisesRegex(ApplyError, "beneath"):
            apply_from_path(
                self.root,
                str(result_path),
                selected_ids=["state-escape"],
                state_root=self.root / ".." / outside.name,
            )
        self.assertFalse(outside.exists())
        valid_state = self.root / "valid-state"
        valid = self._apply(result_path, "state-escape", state_root=valid_state)
        self.assertEqual(valid["status"], "completed")
        self.assertTrue(valid_state.is_dir())
        target = self.root / "real-state"
        target.mkdir()
        for name, state_root in (
            ("existing-state-link", target),
            ("dangling-state-link", self.root / "missing-state"),
        ):
            link = self.root / name
            link.symlink_to(state_root, target_is_directory=True)
            with self.assertRaisesRegex(ApplyError, "symlink"):
                self._apply(result_path, "state-escape", state_root=link)

    def test_interruption_before_replacement_is_resumable(self) -> None:
        item = self._item("interrupt")
        result_path = self._result([item])
        cancellation = Event()

        interrupted = self._apply(
            result_path,
            "interrupt",
            cancellation_event=cancellation,
            before_replace=self._cancel_before_replace(cancellation),
        )

        self.assertEqual(interrupted["status"], "interrupted")
        self.assertEqual(interrupted["items"][0]["resolution"], "incomplete")
        self.assertFalse((self.root / "out/interrupt.txt").exists())
        resumed = self._apply(result_path, "interrupt")
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
