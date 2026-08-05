"""Pure archive-inspector receipt parsing and ranking for the Rust profiler."""

from __future__ import annotations

import json
import os
from contextlib import contextmanager
from pathlib import Path


SCHEMA = "codexy.archive-inspector.receipt/v1"
FIELDS = {
    "schema": str,
    "id": str,
    "test": str,
    "fixture": str,
    "backend": str,
    "started_epoch_us": int,
    "ended_epoch_us": int,
    "duration_us": int,
    "inspector_outcome": str,
    "content_comparator_ran": bool,
}
GROUP_FIELDS = (
    "test",
    "fixture",
    "backend",
    "inspector_outcome",
    "content_comparator_ran",
)
RECEIPT_DIRECTORY_ENV = "CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR"


@contextmanager
def receipt_environment(root: Path):
    directory = root / "archive-inspector-receipts"
    directory.mkdir()
    previous = os.environ.get(RECEIPT_DIRECTORY_ENV)
    os.environ[RECEIPT_DIRECTORY_ENV] = str(directory)
    try:
        yield directory, os.environ.copy()
    finally:
        if previous is None:
            del os.environ[RECEIPT_DIRECTORY_ENV]
        else:
            os.environ[RECEIPT_DIRECTORY_ENV] = previous


def receipt_report(directory: Path) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    receipts = load_archive_inspection_receipts(directory)
    return receipts, rank_archive_inspection_receipts(receipts)


def receipt_report_lines(directory: Path) -> tuple[str, str]:
    receipts, ranked = receipt_report(directory)
    return (
        "archive-inspector-receipts-json\t" + json.dumps(receipts, sort_keys=True),
        "archive-inspector-rank-json\t" + json.dumps(ranked, sort_keys=True),
    )


def emit_receipt_report(phase: object) -> None:
    lines = phase if isinstance(phase, tuple) else ()
    print(*lines, sep="\n")


def load_archive_inspection_receipts(directory: Path) -> list[dict[str, object]]:
    receipts = []
    for path in sorted(directory.glob("*.json")):
        try:
            receipt = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue
        if valid_receipt(receipt):
            receipts.append(receipt)
    return sorted(
        receipts,
        key=lambda receipt: (
            str(receipt["test"]),
            str(receipt["fixture"]),
            str(receipt["backend"]),
            int(receipt["started_epoch_us"]),
            str(receipt["id"]),
        ),
    )


def rank_archive_inspection_receipts(receipts: list[dict[str, object]]) -> list[dict[str, object]]:
    groups: dict[tuple[object, ...], dict[str, object]] = {}
    for receipt in receipts:
        key = tuple(receipt[name] for name in GROUP_FIELDS)
        group = groups.setdefault(
            key,
            {**dict(zip(GROUP_FIELDS, key)), "invocations": 0, "total_duration_us": 0, "max_duration_us": 0},
        )
        duration = int(receipt["duration_us"])
        group["invocations"] = int(group["invocations"]) + 1
        group["total_duration_us"] = int(group["total_duration_us"]) + duration
        group["max_duration_us"] = max(int(group["max_duration_us"]), duration)
    return sorted(
        groups.values(),
        key=lambda group: (
            -int(group["total_duration_us"]),
            *(str(group[name]) for name in GROUP_FIELDS),
        ),
    )


def valid_receipt(receipt: object) -> bool:
    return (
        isinstance(receipt, dict)
        and receipt.get("schema") == SCHEMA
        and all(type(receipt.get(name)) is expected for name, expected in FIELDS.items())
    )
