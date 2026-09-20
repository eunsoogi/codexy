"""Per-item reuse decisions and checkpointed runner execution."""

from __future__ import annotations

import os
import time
from pathlib import Path
from threading import Event
from typing import Any, Callable, Mapping

from execution import run_item

from resume_errors import ResumeError
from result_validation import public_item, safe_output_path, saved_result_is_reusable
from state import StateError, file_state
from persistence import persist_state, sync_file


def execute_items(
    items: list[Mapping[str, Any]],
    state: dict[str, Any],
    *,
    state_path: Path,
    root: Path,
    results: Path,
    run_root: Path,
    artifact_root: Path,
    operation: Mapping[str, Any],
    current_input_identity: str,
    output_limit_bytes: int,
    cancellation_event: Event | None,
    persistence_hook: Callable[[str, Path], None] | None,
) -> tuple[list[dict[str, Any]], str]:
    public_items: list[dict[str, Any]] = []
    top_status = "completed"
    for index, item in enumerate(items):
        item_id = str(item["id"])
        saved = state["items"][item_id]
        identity = operation["item_identities"][item_id]
        reusable, reason = saved_result_is_reusable(
            saved,
            item,
            workspace=root,
            results_root=results,
            current_item_identity=identity,
            current_input_identity=current_input_identity,
        )
        if reusable:
            public_items.append(
                public_item(
                    item,
                    resolution="reuse",
                    invocations=saved["invocations"],
                    runner_result=saved["result"]["runner"],
                    reason=reason,
                )
            )
            continue
        if reason == "original-changed" and saved.get("status") == "succeeded":
            saved.update(
                {
                    "spec_identity": identity,
                    "input_identity": current_input_identity,
                    "status": "conflict",
                    "result": None,
                    "reason": reason,
                }
            )
            top_status = "conflict"
            persist_state(state_path, state, persistence_hook)
            public_items.append(
                public_item(
                    item,
                    resolution="conflict",
                    invocations=saved["invocations"],
                    reason=reason,
                )
            )
            continue
        saved.update(
            {
                "spec_identity": identity,
                "input_identity": current_input_identity,
                "status": "in-progress",
                "invocations": saved["invocations"] + 1,
                "result": None,
                "owner_pid": os.getpid(),
                "started_at_ns": time.time_ns(),
            }
        )
        persist_state(state_path, state, persistence_hook)
        result, control = run_item(
            item,
            index=index,
            workspace=root,
            run_root=run_root,
            artifact_root=artifact_root,
            output_limit_bytes=output_limit_bytes,
            cancellation_event=cancellation_event,
        )
        failure = result.get("failure")
        if isinstance(failure, Mapping) and failure.get("reason") == "original-changed":
            saved["status"] = "conflict"
            saved["reason"] = "original-changed"
            top_status = "conflict"
        elif control == "abort":
            saved["status"] = "conflict"
            saved["result"] = {"runner": result}
            saved["reason"] = (
                str(failure.get("reason"))
                if isinstance(failure, Mapping) and failure.get("reason")
                else "runner-aborted"
            )
            top_status = "conflict"
        elif result.get("status") == "succeeded":
            output = result.get("output")
            artifact_path = safe_output_path(
                results,
                output.get("path") if isinstance(output, Mapping) else None,
            )
            if artifact_path is None:
                raise ResumeError(
                    "runner returned an artifact outside the results root"
                )
            try:
                artifact_state = file_state(artifact_path, "validated artifact")
                sync_file(artifact_path, "validated artifact")
            except StateError as error:
                raise ResumeError(str(error)) from error
            saved["status"] = "succeeded"
            saved["result"] = {"runner": result, "artifact_state": artifact_state}
        else:
            saved["status"] = "failed"
            saved["result"] = {"runner": result}
            if control == "cancel":
                top_status = "interrupted"
        saved.pop("owner_pid", None)
        saved.pop("started_at_ns", None)
        persist_state(state_path, state, persistence_hook)
        resolution = "conflict" if saved["status"] == "conflict" else "rerun"
        public_items.append(
            public_item(
                item,
                resolution=resolution,
                invocations=saved["invocations"],
                runner_result=None if resolution == "conflict" else result,
                reason=saved.get("reason") if resolution == "conflict" else reason,
            )
        )
        if control == "cancel":
            top_status = "interrupted"
            for pending_item in items[index + 1 :]:
                public_items.append(
                    public_item(
                        pending_item,
                        resolution="pending",
                        invocations=state["items"][str(pending_item["id"])][
                            "invocations"
                        ],
                        reason="cancelled",
                    )
                )
            break
        if control == "abort":
            top_status = "conflict"
    return public_items, top_status
