"""Schema validation for persisted batch resume state."""

from __future__ import annotations

from typing import Any, Mapping

from constants import ITEM_STATUSES, RESUME_SCHEMA
from resume_errors import ResumeError


def validate_state(
    value: Mapping[str, Any], batch_id: str, workspace: str
) -> dict[str, Any]:
    if set(value) != {"schema", "batch_id", "workspace", "operation", "items"}:
        raise ResumeError("resume state has an invalid shape")
    if (
        value.get("schema") != "codexy.batch-change-resume-state.v1"
        or value.get("batch_id") != batch_id
    ):
        raise ResumeError("resume state belongs to a different batch")
    if value.get("workspace") != workspace:
        raise ResumeError("resume state belongs to a different workspace")
    operation = value.get("operation")
    items = value.get("items")
    if not isinstance(operation, Mapping) or not isinstance(items, Mapping):
        raise ResumeError("resume state has invalid operation or items")
    required_operation = {
        "schema",
        "workspace",
        "results_root",
        "input_identity",
        "environment_identity",
        "output_limit_bytes",
        "item_identities",
        "identity",
    }
    if set(operation) != required_operation or operation.get("schema") != RESUME_SCHEMA:
        raise ResumeError("resume state has an invalid operation shape")
    if operation.get("workspace") != workspace or not isinstance(
        operation.get("results_root"), str
    ):
        raise ResumeError("resume state has an invalid operation workspace")
    if (
        not isinstance(operation.get("input_identity"), str)
        or not isinstance(operation.get("environment_identity"), str)
        or not isinstance(operation.get("identity"), str)
        or isinstance(operation.get("output_limit_bytes"), bool)
        or not isinstance(operation.get("output_limit_bytes"), int)
        or not 0 < operation["output_limit_bytes"] <= 64 * 1024 * 1024
    ):
        raise ResumeError("resume state has invalid operation identities")
    if not isinstance(operation.get("item_identities"), Mapping) or any(
        not isinstance(item_id, str) or not isinstance(identity, str)
        for item_id, identity in operation["item_identities"].items()
    ):
        raise ResumeError("resume state has invalid item identities")
    base_fields = {"spec_identity", "input_identity", "status", "invocations", "result"}
    optional_fields = {"owner_pid", "started_at_ns", "reason"}
    for item_id, saved in items.items():
        if not isinstance(item_id, str) or not isinstance(saved, Mapping):
            raise ResumeError("resume state has invalid item entries")
        if not set(saved).issubset(
            base_fields | optional_fields
        ) or not base_fields.issubset(saved):
            raise ResumeError(f"resume state item {item_id} has an invalid shape")
        if saved.get("status") not in ITEM_STATUSES:
            raise ResumeError(f"resume state item {item_id} has an invalid status")
        if (
            isinstance(saved.get("invocations"), bool)
            or not isinstance(saved.get("invocations"), int)
            or saved["invocations"] < 0
        ):
            raise ResumeError(
                f"resume state item {item_id} has an invalid invocation count"
            )
        for field in ("owner_pid", "started_at_ns"):
            if field in saved and (
                isinstance(saved[field], bool)
                or not isinstance(saved[field], int)
                or saved[field] < 0
            ):
                raise ResumeError(f"resume state item {item_id} has an invalid {field}")
        if "reason" in saved and not isinstance(saved["reason"], str):
            raise ResumeError(f"resume state item {item_id} has an invalid reason")
        if not isinstance(saved.get("spec_identity"), str) or not isinstance(
            saved.get("input_identity"), str
        ):
            raise ResumeError(f"resume state item {item_id} has invalid identities")
        if saved.get("status") == "succeeded" and not isinstance(
            saved.get("result"), Mapping
        ):
            raise ResumeError(f"resume state item {item_id} has no saved result")
        if (
            saved.get("status") != "succeeded"
            and saved.get("result") is not None
            and not isinstance(saved.get("result"), Mapping)
        ):
            raise ResumeError(f"resume state item {item_id} has an invalid result")
    return dict(value)
