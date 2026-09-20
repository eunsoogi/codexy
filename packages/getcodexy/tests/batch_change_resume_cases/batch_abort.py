"""Batch-wide abort and original-invariance scenarios."""

from __future__ import annotations

from pathlib import Path

from .common import BatchChangeResumeCase, preview, resume_batch


BATCH_ABORT_COMMAND_SOURCE = """
from pathlib import Path
import sys

mode, args = sys.argv[1], sys.argv[2:]

def record(path, value):
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open("a", encoding="utf-8") as output:
        output.write(value + "\\n")

if mode == "mutate-stage":
    record(args[2], mode)
    Path(args[0]).write_text("contract violation", encoding="utf-8")
elif mode in {"copy", "mutate-original"}:
    source, destination, log = args[:3]
    record(log, mode)
    Path(destination).write_text(
        Path(source).read_text(encoding="utf-8").upper(), encoding="utf-8"
    )
    if mode == "mutate-original":
        Path(args[3]).write_text("changed by later item\\n", encoding="utf-8")
elif mode == "validate":
    output, log = args[:2]
    record(log, mode)
    raise SystemExit(0 if Path(output).read_text(encoding="utf-8").isupper() else 9)
else:
    raise SystemExit(2)
"""


class BatchChangeResumeBatchAbortTests(BatchChangeResumeCase):
    def _batch_item(
        self,
        *,
        item_id: str,
        original: str,
        output: str,
        mode: str,
        extra_args: tuple[object, ...] = (),
    ) -> dict[str, object]:
        (self.root / original).write_text("hello\n", encoding="utf-8")
        transform_args = (mode, original, output, self.transform_log, *extra_args)
        return {
            "id": item_id,
            "original": original,
            "output": output,
            "transform": {
                "argv": self._argv(*transform_args),
                "timeout_seconds": 5,
            },
            "validations": [
                {
                    "argv": self._argv("validate", output, self.validation_log),
                    "timeout_seconds": 5,
                }
            ],
        }

    def _run_items(self, items: list[dict[str, object]]) -> dict[str, object]:
        document = preview(self.root, {"items": items})
        return resume_batch(document, workspace_root=self.root)

    def setUp(self) -> None:
        super().setUp()
        self.command.write_text(BATCH_ABORT_COMMAND_SOURCE, encoding="utf-8")

    def test_contract_abort_does_not_run_later_items(self) -> None:
        first = self._batch_item(
            item_id="first",
            original="input.txt",
            output="first-output.txt",
            mode="mutate-stage",
        )
        second = self._batch_item(
            item_id="second",
            original="second.txt",
            output="second-output.txt",
            mode="copy",
        )

        result = self._run_items([first, second])

        self.assertEqual(result["status"], "conflict")
        self.assertEqual(
            [item["resolution"] for item in result["items"]],
            ["conflict", "pending"],
        )
        self.assertEqual(self._log_count(self.transform_log, "copy"), 0)
        self.assertEqual(
            [item["status"] for item in result["items"]],
            ["conflict", "not-run"],
        )

    def test_later_original_change_aborts_the_remaining_batch(self) -> None:
        first = self._batch_item(
            item_id="first",
            original="input.txt",
            output="first-output.txt",
            mode="copy",
        )
        second = self._batch_item(
            item_id="second",
            original="second.txt",
            output="second-output.txt",
            mode="mutate-original",
            extra_args=(str(self.original),),
        )

        result = self._run_items([first, second])

        self.assertEqual(result["status"], "conflict")
        self.assertEqual(result["items"][0]["resolution"], "rerun")
        self.assertEqual(result["items"][0]["status"], "succeeded")
        self.assertEqual(result["items"][1]["resolution"], "conflict")
        self.assertEqual(result["items"][1]["reason"], "original-changed")
        self.assertEqual(self._log_count(self.transform_log, "copy"), 1)
        self.assertEqual(self._log_count(self.transform_log, "mutate-original"), 1)

    def test_original_change_between_preview_and_execution_aborts_batch(self) -> None:
        first = self._batch_item(
            item_id="first",
            original="input.txt",
            output="first-output.txt",
            mode="copy",
        )
        second = self._batch_item(
            item_id="second",
            original="second.txt",
            output="second-output.txt",
            mode="copy",
        )
        document = preview(self.root, {"items": [first, second]})
        self.original.write_text("changed after preview\n", encoding="utf-8")

        result = resume_batch(document, workspace_root=self.root)

        self.assertEqual(result["status"], "conflict")
        self.assertEqual(
            [item["resolution"] for item in result["items"]],
            ["conflict", "pending"],
        )
        self.assertEqual(self._log_count(self.transform_log, "copy"), 0)
