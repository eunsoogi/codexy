"""Per-item command execution and result handling."""

from __future__ import annotations

import shutil
from pathlib import Path
from threading import Event
from typing import Any, Mapping

from errors import RunnerError
from processes import run_command
from staging import (
    _artifact,
    _copy_input,
    _file_state,
    _relative_path,
    _same_state,
    _stage_output,
    _unchanged,
    all_originals_unchanged,
)


def _item_result(item: Mapping[str, Any]) -> dict[str, Any]:
    original = item["original"]
    output = item["output"]
    return {
        "id": item["id"],
        "status": "failed",
        "original": {"path": original["path"], "state": original["state"]},
        "output_path": output["path"],
        "output": None,
        "execution": {"transform": None, "validations": []},
        "failure": None,
    }


def _failure(
    result: dict[str, Any],
    *,
    phase: str,
    reason: str,
) -> dict[str, Any]:
    result["failure"] = {"phase": phase, "reason": reason}
    return result


def _command_failure(command: Mapping[str, Any]) -> str:
    state = command.get("state")
    if state in {"timeout", "cancelled", "output-limit", "spawn-failed"}:
        return str(state)
    return "nonzero-exit"


def _stop(
    result: dict[str, Any], phase: str, reason: str, control: str
) -> tuple[dict[str, Any], str]:
    return _failure(result, phase=phase, reason=reason), control


def run_item(
    item: Mapping[str, Any],
    *,
    index: int,
    workspace: Path,
    run_root: Path,
    artifact_root: Path,
    output_limit_bytes: int,
    cancellation_event: Event | None,
) -> tuple[dict[str, Any], str]:
    result = _item_result(item)
    try:
        original_relative = _relative_path(
            workspace, item["original"]["path"], "original"
        )
    except (OSError, RunnerError):
        return _stop(result, "preflight", "original-changed", "abort")
    try:
        output_relative = _relative_path(workspace, item["output"]["path"], "output")
    except (OSError, RunnerError):
        return _stop(result, "preflight", "contract-violation", "abort")
    expected_state = item["original"]["state"]
    if not _unchanged(workspace / original_relative, expected_state):
        return _stop(result, "preflight", "original-changed", "abort")

    stage = run_root / f"work-{index:04d}"
    try:
        stage.mkdir()
        stage_original = _copy_input(workspace, stage, original_relative)
        initial_stage_state = _file_state(stage_original)
    except (OSError, RunnerError):
        shutil.rmtree(stage, ignore_errors=True)
        return _stop(result, "preflight", "original-changed", "abort")
    if not _same_state(expected_state, initial_stage_state):
        shutil.rmtree(stage, ignore_errors=True)
        return _stop(result, "preflight", "original-changed", "abort")

    try:
        stage_output = _stage_output(stage, output_relative, fresh=True)
    except RunnerError:
        return _stop(result, "preflight", "contract-violation", "abort")

    try:
        transform = item["commands"]["transform"]
        transform_result = run_command(
            transform["argv"],
            cwd=stage,
            timeout_seconds=transform["timeout_seconds"],
            output_limit_bytes=output_limit_bytes,
            cancellation_event=cancellation_event,
        )
        result["execution"]["transform"] = transform_result
        if not _unchanged(stage_original, initial_stage_state):
            return _stop(result, "transform", "contract-violation", "abort")
        if not _unchanged(workspace / original_relative, expected_state):
            return _stop(result, "transform", "original-changed", "abort")
        if transform_result["state"] != "exited" or transform_result["exit_code"] != 0:
            reason = _command_failure(transform_result)
            control = "cancel" if reason == "cancelled" else "continue"
            return _stop(result, "transform", reason, control)
        if cancellation_event is not None and cancellation_event.is_set():
            return _stop(result, "batch", "cancelled", "cancel")
        try:
            stage_output = _stage_output(stage, output_relative)
        except RunnerError:
            return _stop(result, "transform", "contract-violation", "abort")
        for validation in item["commands"]["validations"]:
            validation_result = run_command(
                validation["argv"],
                cwd=stage,
                timeout_seconds=validation["timeout_seconds"],
                output_limit_bytes=output_limit_bytes,
                cancellation_event=cancellation_event,
            )
            result["execution"]["validations"].append(validation_result)
            if not _unchanged(stage_original, initial_stage_state):
                return _stop(result, "validation", "contract-violation", "abort")
            if not _unchanged(workspace / original_relative, expected_state):
                return _stop(result, "validation", "original-changed", "abort")
            if (
                validation_result["state"] != "exited"
                or validation_result["exit_code"] != 0
            ):
                reason = _command_failure(validation_result)
                control = "cancel" if reason == "cancelled" else "continue"
                return _stop(result, "validation", reason, control)
        if cancellation_event is not None and cancellation_event.is_set():
            return _stop(result, "batch", "cancelled", "cancel")
        if not _unchanged(workspace / original_relative, expected_state):
            return _stop(result, "validation", "original-changed", "abort")
        if not _unchanged(stage_original, initial_stage_state):
            return _stop(result, "validation", "contract-violation", "abort")
        try:
            stage_output = _stage_output(stage, output_relative)
        except RunnerError:
            return _stop(result, "validation", "contract-violation", "abort")
        try:
            artifact_path, artifact_state = _artifact(
                stage_output, artifact_root, index, output_relative
            )
        except (OSError, RunnerError):
            return _stop(result, "transform", "contract-violation", "abort")
        result["status"] = "succeeded"
        result["output"] = {
            "path": str(artifact_path),
            "relative_path": str(output_relative),
            "state": artifact_state,
            "validated": True,
        }
        return result, "continue"
    except (OSError, RunnerError):
        return _stop(result, "runner", "artifact-error", "continue")
    finally:
        shutil.rmtree(stage, ignore_errors=True)
