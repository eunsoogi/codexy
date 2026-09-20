"""Batch-runner orchestration and the public input-path API."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path
from threading import Event
from typing import Any, Mapping

from errors import RunnerError
from execution import all_originals_unchanged, run_item
from processes import (
    DEFAULT_OUTPUT_LIMIT_BYTES,
    SUPPORTED_PLATFORM,
    validate_output_limit,
)


INPUT_SCHEMA = "codexy.batch-change-input.v1"
RUNNER_SCHEMA = "codexy.batch-change-runner.v1"


def _absolute_directory(value: str | Path, label: str) -> Path:
    path = Path(value).expanduser().absolute()
    if path.exists() and path.is_symlink():
        raise RunnerError(f"{label} must not be a symlink")
    if path.exists() and not path.is_dir():
        raise RunnerError(f"{label} must be a directory")
    path.mkdir(parents=True, exist_ok=True)
    return path.resolve(strict=True)


def _validate_input(preview: Mapping[str, Any]) -> list[Mapping[str, Any]]:
    if preview.get("schema") != INPUT_SCHEMA or preview.get("status") != "preview":
        raise RunnerError("runner requires a validated batch-change preview")
    items = preview.get("items")
    if not isinstance(items, list) or not items:
        raise RunnerError("validated preview must contain items")
    for index, item in enumerate(items):
        if not isinstance(item, Mapping) or not isinstance(item.get("id"), str):
            raise RunnerError(f"preview item {index} is invalid")
        original = item.get("original")
        output = item.get("output")
        commands = item.get("commands")
        if not isinstance(original, Mapping) or not isinstance(output, Mapping):
            raise RunnerError(f"preview item {index} is missing file details")
        if not isinstance(original.get("path"), str) or not isinstance(
            original.get("state"), Mapping
        ):
            raise RunnerError(f"preview item {index} has invalid original state")
        if not isinstance(output.get("path"), str) or not isinstance(commands, Mapping):
            raise RunnerError(f"preview item {index} has invalid output details")
        transform = commands.get("transform")
        validations = commands.get("validations")
        if not isinstance(transform, Mapping) or not isinstance(validations, list):
            raise RunnerError(f"preview item {index} has invalid commands")
        if not validations:
            raise RunnerError(f"preview item {index} has no validations")
    return items


def _item_result(item: Mapping[str, Any], status: str, reason: str) -> dict[str, Any]:
    original = item["original"]
    output = item["output"]
    return {
        "id": item["id"],
        "status": status,
        "original": {"path": original["path"], "state": original["state"]},
        "output_path": output["path"],
        "output": None,
        "execution": {"transform": None, "validations": []},
        "failure": {"phase": "batch", "reason": reason},
    }


def _result(
    *,
    status: str,
    workspace: Path,
    artifact_root: Path,
    items: list[dict[str, Any]],
    total: int,
) -> dict[str, Any]:
    return {
        "schema": RUNNER_SCHEMA,
        "status": status,
        "platform": "posix-process-group",
        "workspace": {"path": str(workspace)},
        "artifacts": {"root": str(artifact_root)},
        "items": items,
        "item_count": total,
        "succeeded_count": sum(item["status"] == "succeeded" for item in items),
        "failed_count": sum(item["status"] == "failed" for item in items),
    }


def run_batch(
    preview: Mapping[str, Any],
    *,
    workspace_root: str | Path,
    results_root: str | Path,
    output_limit_bytes: int = DEFAULT_OUTPUT_LIMIT_BYTES,
    cancellation_event: Event | None = None,
) -> dict[str, Any]:
    """Run a validated preview sequentially and retain only validated outputs."""
    if not SUPPORTED_PLATFORM:
        raise RunnerError("batch runner requires POSIX process-group support")
    validate_output_limit(output_limit_bytes)
    items = _validate_input(preview)
    workspace = _absolute_directory(workspace_root, "workspace root")
    results = _absolute_directory(results_root, "results root")
    run_root = Path(tempfile.mkdtemp(prefix="batch-", dir=results))
    artifact_root = run_root / "artifacts"
    artifact_root.mkdir()
    item_results: list[dict[str, Any]] = []
    if not all_originals_unchanged(workspace, items):
        item_results.append(_item_result(items[0], "failed", "original-changed"))
        item_results.extend(
            _item_result(item, "not-run", "batch-aborted") for item in items[1:]
        )
        return _result(
            status="aborted",
            workspace=workspace,
            artifact_root=artifact_root,
            items=item_results,
            total=len(items),
        )

    status = "completed"
    for index, item in enumerate(items):
        if cancellation_event is not None and cancellation_event.is_set():
            status = "cancelled"
            item_results.extend(
                _item_result(item, "not-run", "cancelled") for item in items[index:]
            )
            break
        result, control = run_item(
            item,
            index=index,
            workspace=workspace,
            run_root=run_root,
            artifact_root=artifact_root,
            output_limit_bytes=output_limit_bytes,
            cancellation_event=cancellation_event,
        )
        item_results.append(result)
        if not all_originals_unchanged(workspace, items):
            result["status"] = "failed"
            result["output"] = None
            result["failure"] = {"phase": "batch", "reason": "original-changed"}
            status = "aborted"
            item_results.extend(
                _item_result(item, "not-run", "batch-aborted")
                for item in items[index + 1 :]
            )
            break
        if control == "cancel":
            status = "cancelled"
            item_results.extend(
                _item_result(item, "not-run", "cancelled")
                for item in items[index + 1 :]
            )
            break
        if control == "abort":
            status = "aborted"
            item_results.extend(
                _item_result(item, "not-run", "batch-aborted")
                for item in items[index + 1 :]
            )
            break
    return _result(
        status=status,
        workspace=workspace,
        artifact_root=artifact_root,
        items=item_results,
        total=len(items),
    )


def run_from_path(
    workspace_root: str | Path,
    input_path: str,
    *,
    results_root: str | Path,
    output_limit_bytes: int = DEFAULT_OUTPUT_LIMIT_BYTES,
    cancellation_event: Event | None = None,
) -> dict[str, Any]:
    """Load a #1140 preview through its read-only parser and execute it."""
    input_scripts = Path(__file__).parents[1] / "batch_change_input"
    if str(input_scripts) not in sys.path:
        sys.path.insert(0, str(input_scripts))
    from manifest import preview_from_path

    preview = preview_from_path(workspace_root, input_path)
    return run_batch(
        preview,
        workspace_root=workspace_root,
        results_root=results_root,
        output_limit_bytes=output_limit_bytes,
        cancellation_event=cancellation_event,
    )


run = run_batch
