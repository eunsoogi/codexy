"""Provenance identity checks for runtime release receipts."""

from typing import Any

from .identity import object


REPOSITORY = "https://github.com/eunsoogi/codexy"
REPOSITORY_ID = 1_269_350_143
PROVENANCE_WORKFLOW = ".github/workflows/runtime-candidate.yml"


def validate_provenance(value: Any) -> dict[str, Any]:
    value = object(value, "provenance")
    if set(value) != {
        "repositoryId",
        "workflowPath",
        "runId",
        "runAttempt",
        "workflowRunUrl",
    }:
        raise ValueError("runtime release provenance has unknown or missing fields")
    if value.get("repositoryId") != REPOSITORY_ID:
        raise ValueError("runtime release provenance repository is not canonical")
    if value.get("workflowPath") != PROVENANCE_WORKFLOW:
        raise ValueError("runtime release provenance workflow is not canonical")
    for field in ("runId", "runAttempt"):
        if type(value.get(field)) is not int or value[field] <= 0:
            raise ValueError(f"runtime release provenance {field} must be positive")
    if value.get("workflowRunUrl") != f"{REPOSITORY}/actions/runs/{value['runId']}":
        raise ValueError("runtime release provenance URL is not canonical")
    return value
