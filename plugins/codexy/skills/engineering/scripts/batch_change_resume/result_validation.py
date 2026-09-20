"""Joint validation of saved runner results, originals, and artifacts."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Mapping

from staging import _relative_path
from state import StateError, file_state, state_matches


def safe_output_path(results_root: Path, path_value: Any) -> Path | None:
    if not isinstance(path_value, str) or not path_value:
        return None
    path = Path(path_value).expanduser().absolute()
    try:
        relative = path.relative_to(results_root)
    except ValueError:
        return None
    current = results_root
    for component in relative.parts:
        current /= component
        if current.is_symlink():
            return None
    return path


def runner_success(result: Mapping[str, Any], item: Mapping[str, Any]) -> bool:
    if result.get("status") != "succeeded" or result.get("id") != item.get("id"):
        return False
    original = result.get("original")
    expected_original = item.get("original")
    if (
        not isinstance(original, Mapping)
        or not isinstance(expected_original, Mapping)
        or original.get("path") != expected_original.get("path")
        or original.get("state") != expected_original.get("state")
    ):
        return False
    if result.get("output_path") != item.get("output", {}).get("path"):
        return False
    output = result.get("output")
    execution = result.get("execution")
    if not isinstance(output, Mapping) or output.get("validated") is not True:
        return False
    if not isinstance(execution, Mapping):
        return False
    transform = execution.get("transform")
    validations = execution.get("validations")
    if (
        not isinstance(transform, Mapping)
        or transform.get("state") != "exited"
        or transform.get("exit_code") != 0
    ):
        return False
    if not isinstance(validations, list) or not validations:
        return False
    return all(
        isinstance(validation, Mapping)
        and validation.get("state") == "exited"
        and validation.get("exit_code") == 0
        for validation in validations
    )


def _existing_file_state(path: Path, label: str) -> dict[str, Any] | None:
    try:
        return file_state(path, label)
    except StateError:
        return None


def saved_result_is_reusable(
    saved: Mapping[str, Any],
    item: Mapping[str, Any],
    *,
    workspace: Path,
    results_root: Path,
    current_item_identity: str,
    current_input_identity: str,
) -> tuple[bool, str]:
    if saved.get("status") != "succeeded":
        return False, "previous-incomplete"
    wrapper = saved.get("result")
    if not isinstance(wrapper, Mapping):
        return False, "state-result-missing"
    runner_result = wrapper.get("runner")
    expected_artifact = wrapper.get("artifact_state")
    if not isinstance(runner_result, Mapping) or not isinstance(
        expected_artifact, Mapping
    ):
        return False, "state-result-corrupt"
    saved_original = runner_result.get("original")
    current_original = item.get("original")
    if (
        isinstance(saved_original, Mapping)
        and isinstance(current_original, Mapping)
        and saved_original.get("state") != current_original.get("state")
    ):
        return False, "original-changed"
    if not runner_success(runner_result, item):
        return False, "result-not-successful"
    original_path = _relative_path(workspace, item["original"]["path"], "original")
    try:
        current_original = file_state(workspace / original_path, "original")
    except StateError:
        return False, "original-changed"
    expected_original_state = {
        key: value for key, value in item["original"]["state"].items() if key != "path"
    }
    if not state_matches(current_original, expected_original_state):
        return False, "original-changed"
    if runner_result.get("original", {}).get("state") != item["original"]["state"]:
        return False, "original-changed"
    if saved.get("spec_identity") != current_item_identity:
        return False, "identity-mismatch"
    if saved.get("input_identity") != current_input_identity:
        return False, "input-changed"
    output = runner_result.get("output")
    artifact_path = safe_output_path(
        results_root, output.get("path") if isinstance(output, Mapping) else None
    )
    if artifact_path is None:
        return False, "artifact-path-invalid"
    actual_artifact = _existing_file_state(artifact_path, "validated artifact")
    if actual_artifact is None or not state_matches(
        actual_artifact, dict(expected_artifact)
    ):
        return False, "artifact-changed"
    if not state_matches(actual_artifact, dict(output.get("state", {}))):
        return False, "artifact-changed"
    return True, "validated-result"


def public_item(
    item: Mapping[str, Any],
    *,
    resolution: str,
    invocations: int,
    runner_result: Mapping[str, Any] | None = None,
    reason: str | None = None,
) -> dict[str, Any]:
    result = {
        "id": item["id"],
        "resolution": resolution,
        "invocations": invocations,
        "status": runner_result.get("status")
        if runner_result
        else ("conflict" if resolution == "conflict" else "not-run"),
        "original": item["original"],
        "output_path": item["output"]["path"],
        "result": dict(runner_result) if runner_result is not None else None,
    }
    if reason is not None:
        result["reason"] = reason
    return result
