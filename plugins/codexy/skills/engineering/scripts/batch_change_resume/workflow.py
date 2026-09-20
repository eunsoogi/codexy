"""The locked, checkpointed batch-change resume workflow."""

from __future__ import annotations

import tempfile
from pathlib import Path
from threading import Event
from typing import Any, Callable, Mapping

from processes import (
    DEFAULT_OUTPUT_LIMIT_BYTES,
    SUPPORTED_PLATFORM,
    validate_output_limit,
)
from runner import _validate_input

from constants import (
    DEFAULT_RESULTS_DIRECTORY,
    DEFAULT_STATE_DIRECTORY,
    RESOLUTION_VALUES,
    RESUME_SCHEMA,
)
from resume_errors import ResumeError
from identity import environment_identity, new_state, operation, stable_batch_id
from item_execution import execute_items
from resume_paths import absolute_directory, workspace
from persistence import batch_lock, persist_state
from state import STATE_SCHEMA, StateError, ensure_child_directory, read_json
from state_validation import validate_state


def resume_batch(
    preview: Mapping[str, Any],
    *,
    workspace_root: str | Path,
    state_root: str | Path | None = None,
    results_root: str | Path | None = None,
    batch_id: str | None = None,
    output_limit_bytes: int = DEFAULT_OUTPUT_LIMIT_BYTES,
    cancellation_event: Event | None = None,
    persistence_hook: Callable[[str, Path], None] | None = None,
) -> dict[str, Any]:
    """Resume one validated preview through explicit reuse/rerun decisions."""
    if not SUPPORTED_PLATFORM:
        raise ResumeError("batch resume requires POSIX process-group support")
    validate_output_limit(output_limit_bytes)
    root = workspace(workspace_root)
    results = absolute_directory(
        results_root or root / DEFAULT_RESULTS_DIRECTORY, "results root"
    )
    try:
        state_directory = ensure_child_directory(
            root, state_root or root / DEFAULT_STATE_DIRECTORY, "state root"
        )
    except StateError as error:
        raise ResumeError(str(error)) from error
    try:
        items = _validate_input(preview)
    except (KeyError, TypeError, ValueError) as error:
        raise ResumeError(
            f"resume requires a validated batch-change preview: {error}"
        ) from error
    preview_workspace = preview.get("workspace")
    if not isinstance(preview_workspace, Mapping) or preview_workspace.get(
        "path"
    ) != str(root):
        raise ResumeError("preview workspace does not match the resume workspace")
    environment = environment_identity(items, root)
    current_operation = operation(
        preview,
        items,
        workspace=root,
        results_root=results,
        environment_identity_value=environment,
        output_limit_bytes=output_limit_bytes,
    )
    current_input_identity = current_operation["input_identity"]
    identifier = stable_batch_id(preview, root, batch_id)
    state_path = state_directory / f"{identifier}.json"
    with batch_lock(state_directory, identifier):
        try:
            loaded = read_json(state_path)
        except StateError as error:
            raise ResumeError(str(error)) from error
        if loaded is None:
            state = new_state(identifier, items, current_operation)
        else:
            state = validate_state(loaded, identifier, str(root))
            saved_items = state["items"]
            for item in items:
                item_id = str(item["id"])
                if item_id not in saved_items:
                    saved_items[item_id] = {
                        "spec_identity": current_operation["item_identities"][item_id],
                        "input_identity": current_input_identity,
                        "status": "pending",
                        "invocations": 0,
                        "result": None,
                    }
                elif item_id not in state["operation"]["item_identities"]:
                    raise ResumeError(
                        f"resume state item {item_id} has no operation identity"
                    )
        state["operation"] = current_operation
        state["workspace"] = str(root)
        persist_state(state_path, state, persistence_hook)
        run_root = Path(tempfile.mkdtemp(prefix="resume-", dir=results))
        artifact_root = run_root / "artifacts"
        artifact_root.mkdir(mode=0o700)
        public_items, top_status = execute_items(
            items,
            state,
            state_path=state_path,
            root=root,
            results=results,
            run_root=run_root,
            artifact_root=artifact_root,
            operation=current_operation,
            current_input_identity=current_input_identity,
            output_limit_bytes=output_limit_bytes,
            cancellation_event=cancellation_event,
            persistence_hook=persistence_hook,
        )
        state["operation"] = current_operation
        persist_state(state_path, state, persistence_hook)
    counts = {
        resolution: sum(item["resolution"] == resolution for item in public_items)
        for resolution in RESOLUTION_VALUES
    }
    return {
        "schema": RESUME_SCHEMA,
        "status": top_status,
        "platform": "posix-process-group",
        "workspace": {"path": str(root)},
        "state": {"path": str(state_path), "schema": STATE_SCHEMA},
        "results": {"root": str(results)},
        "batch_id": identifier,
        "operation": {
            "identity": current_operation["identity"],
            "input_identity": current_input_identity,
        },
        "items": public_items,
        "item_count": len(items),
        "counts": counts,
    }
