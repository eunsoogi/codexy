"""Apply selected successful resume results one output at a time."""

from __future__ import annotations

from pathlib import Path
from threading import Event
from typing import Any, Callable, Mapping

from .errors import ApplyError
from .execute import run
from .paths import child_directory, directory, workspace
from .state import STATE_SCHEMA, lock, new_state, read, validate, write
from .validation import (
    document,
    diff_for,
    items,
    result_item,
    selected_ids,
    successful,
)


APPLY_SCHEMA = "codexy.batch-change-apply.v1"
DEFAULT_STATE_DIRECTORY = ".codexy-batch-apply"


def _state_path(state_root: Path, batch_id: Any) -> Path:
    if not isinstance(batch_id, str) or not batch_id:
        raise ApplyError("resume result has an invalid batch id")
    allowed = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
    if any(character not in allowed for character in batch_id):
        raise ApplyError("resume result has an invalid batch id")
    return state_root / f"{batch_id}.json"


def _document_result(
    status: str,
    root: Path,
    state_path: Path,
    batch_id: Any,
    selected: list[str],
    values: list[dict[str, Any]],
    entrypoint: str | None = None,
) -> dict[str, Any]:
    return {
        "schema": APPLY_SCHEMA,
        "status": status,
        "workspace": {"path": str(root)},
        "state": {"path": str(state_path), "schema": STATE_SCHEMA},
        "batch_id": batch_id,
        "selection": list(selected),
        "items": values,
        "item_count": len(values),
        "counts": {
            value: sum(item["resolution"] == value for item in values)
            for value in (
                "applied",
                "completed",
                "conflict",
                "incomplete",
                "unselected",
                "preview",
            )
        },
        "provenance": {
            "entrypoint": entrypoint or str(Path(__file__).resolve()),
            "module": str(Path(__file__).resolve()),
        },
    }


def _preview(
    values: list[Mapping[str, Any]],
    selected: set[str],
    root: Path,
    results_root: Path,
) -> list[dict[str, Any]]:
    output: list[dict[str, Any]] = []
    for item in values:
        if item["id"] not in selected:
            output.append(
                result_item(
                    item,
                    selected=False,
                    resolution="unselected",
                    status="not-selected",
                )
            )
        elif not successful(item):
            output.append(
                result_item(
                    item,
                    selected=True,
                    resolution="incomplete",
                    status="incomplete",
                    reason="source-not-successful",
                )
            )
        else:
            diff, _, _, _, _ = diff_for(item, root, results_root)
            output.append(
                result_item(
                    item,
                    selected=True,
                    resolution="preview",
                    status="incomplete",
                    reason="preview-only",
                    diff=diff,
                )
            )
    return output


def apply_results(
    document_value: Mapping[str, Any],
    *,
    result_identity: str,
    workspace_root: str | Path,
    selected_values: list[str],
    state_root: str | Path | None = None,
    results_root: str | Path | None = None,
    cancellation_event: Event | None = None,
    dry_run: bool = False,
    before_replace: Callable[[Path], None] | None = None,
    entrypoint: str | None = None,
) -> dict[str, Any]:
    root = workspace(workspace_root)
    values = items(document_value, root)
    selected = selected_ids(values, selected_values)
    source_results = document_value.get("results")
    result_directory_value = results_root
    if result_directory_value is None and isinstance(source_results, Mapping):
        result_directory_value = source_results.get("root")
    if not isinstance(result_directory_value, (str, Path)):
        raise ApplyError("resume result does not identify a results root")
    result_directory = directory(result_directory_value, "results root", create=False)
    state_directory = child_directory(
        root,
        state_root or root / DEFAULT_STATE_DIRECTORY,
        "state root",
    )
    state_path = _state_path(state_directory, document_value.get("batch_id"))
    if dry_run:
        return _document_result(
            "preview",
            root,
            state_path,
            document_value.get("batch_id"),
            selected_values,
            _preview(values, selected, root, result_directory),
            entrypoint,
        )
    with lock(state_directory, state_path.stem):
        loaded = read(state_path)
        state = (
            new_state(
                state_path.stem,
                str(root),
                result_identity,
                [str(item["id"]) for item in values],
            )
            if loaded is None
            else validate(loaded, state_path.stem, str(root), result_identity)
        )
        for item in values:
            state["items"].setdefault(
                str(item["id"]),
                {"status": "pending", "attempts": 0, "reason": None},
            )
        write(state_path, state)
        public, status = run(
            values,
            selected,
            state,
            state_path=state_path,
            root=root,
            results_root=result_directory,
            cancellation_event=cancellation_event,
            before_replace=before_replace,
        )
    return _document_result(
        status,
        root,
        state_path,
        document_value.get("batch_id"),
        selected_values,
        public,
        entrypoint,
    )


def apply_from_path(
    workspace_root: str | Path,
    result_path: str,
    *,
    selected_ids: list[str],
    state_root: str | Path | None = None,
    results_root: str | Path | None = None,
    cancellation_event: Event | None = None,
    dry_run: bool = False,
    before_replace: Callable[[Path], None] | None = None,
    entrypoint: str | None = None,
) -> dict[str, Any]:
    document_value, identity = document(result_path)
    return apply_results(
        document_value,
        result_identity=identity,
        workspace_root=workspace_root,
        selected_values=selected_ids,
        state_root=state_root,
        results_root=results_root,
        cancellation_event=cancellation_event,
        dry_run=dry_run,
        before_replace=before_replace,
        entrypoint=entrypoint,
    )
