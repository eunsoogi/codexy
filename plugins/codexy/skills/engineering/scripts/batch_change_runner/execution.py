"""Per-item staging, artifact handling, and invariance checks."""

from __future__ import annotations

import hashlib
import os
import shutil
import stat
from pathlib import Path, PureWindowsPath
from threading import Event
from typing import Any, Mapping

from errors import RunnerError
from processes import run_command


def _relative_path(root: Path, value: Any, label: str) -> Path:
    if not isinstance(value, str) or not value:
        raise RunnerError(f"{label} must be a non-empty relative path")
    native = Path(value)
    windows = PureWindowsPath(value)
    if native.is_absolute() or windows.is_absolute() or windows.drive:
        raise RunnerError(f"{label} must be relative to the workspace")
    current = root
    for position, component in enumerate(native.parts):
        if component == ".":
            continue
        current = current.parent if component == ".." else current / component
        try:
            current.relative_to(root)
        except ValueError as error:
            raise RunnerError(f"{label} escapes the workspace") from error
        if current.is_symlink():
            raise RunnerError(f"{label} crosses a symlink")
        if (
            position < len(native.parts) - 1
            and current.exists()
            and not current.is_dir()
        ):
            raise RunnerError(f"{label} crosses a non-directory")
    candidate = (root / native).resolve(strict=False)
    try:
        return candidate.relative_to(root)
    except ValueError as error:
        raise RunnerError(f"{label} escapes the workspace") from error


def _file_state(path: Path) -> dict[str, object]:
    if path.is_symlink() or not path.is_file():
        raise RunnerError("expected a regular file")
    before = path.stat()
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    after = path.stat()
    before_state = (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
    after_state = (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
    if before_state != after_state:
        raise RunnerError("file changed while it was being read")
    return {"size": before.st_size, "mtime_ns": before.st_mtime_ns, "sha256": digest}


def _same_state(expected: Mapping[str, Any], actual: Mapping[str, Any]) -> bool:
    return expected.get("sha256") == actual.get("sha256")


def _unchanged(path: Path, expected: Mapping[str, Any]) -> bool:
    try:
        return _same_state(expected, _file_state(path))
    except RunnerError:
        return False


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


def _copy_input(workspace: Path, stage: Path, relative: Path) -> Path:
    source = workspace / relative
    destination = stage / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    with os.fdopen(os.open(source, os.O_RDONLY | os.O_NOFOLLOW), "rb") as source_file:
        if not stat.S_ISREG(os.fstat(source_file.fileno()).st_mode):
            raise RunnerError("original is not a regular file")
        destination_descriptor = os.open(
            destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
        )
        with os.fdopen(destination_descriptor, "wb") as destination_file:
            shutil.copyfileobj(source_file, destination_file, 1024 * 1024)
    return destination


def all_originals_unchanged(workspace: Path, items: list[Mapping[str, Any]]) -> bool:
    for item in items:
        try:
            relative = _relative_path(workspace, item["original"]["path"], "original")
        except RunnerError:
            return False
        if not _unchanged(workspace / relative, item["original"]["state"]):
            return False
    return True


def _artifact(
    stage_output: Path,
    artifact_root: Path,
    index: int,
    relative: Path,
) -> tuple[Path, dict[str, object]]:
    if stage_output.is_symlink() or not stage_output.is_file():
        raise RunnerError("transform did not leave a regular output file")
    destination = artifact_root / f"item-{index:04d}" / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(stage_output, destination)
    return destination, _file_state(destination)


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
    except RunnerError:
        return _stop(result, "preflight", "original-changed", "abort")
    try:
        output_relative = _relative_path(workspace, item["output"]["path"], "output")
    except RunnerError:
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
        transform = item["commands"]["transform"]
        transform_result = run_command(
            transform["argv"],
            cwd=stage,
            timeout_seconds=transform["timeout_seconds"],
            output_limit_bytes=output_limit_bytes,
            cancellation_event=cancellation_event,
        )
        result["execution"]["transform"] = transform_result
        if transform_result["state"] != "exited" or transform_result["exit_code"] != 0:
            reason = _command_failure(transform_result)
            control = "cancel" if reason == "cancelled" else "continue"
            return _stop(result, "transform", reason, control)
        if cancellation_event is not None and cancellation_event.is_set():
            return _stop(result, "batch", "cancelled", "cancel")
        if not _same_state(initial_stage_state, _file_state(stage_original)):
            return _stop(result, "transform", "contract-violation", "abort")
        if not _unchanged(workspace / original_relative, expected_state):
            return _stop(result, "transform", "original-changed", "abort")

        stage_output = stage / output_relative
        if stage_output.is_symlink() or not stage_output.is_file():
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
        if not _same_state(initial_stage_state, _file_state(stage_original)):
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
