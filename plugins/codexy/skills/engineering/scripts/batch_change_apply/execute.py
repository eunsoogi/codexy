"""Checkpointed per-item application execution."""

from __future__ import annotations

from pathlib import Path
from threading import Event
from typing import Any, Callable, Mapping

from .errors import ApplyError, ApplyInterrupted
from .replace import apply_one
from .state import write
from .validation import diff_for, result_item, successful


def run(
    values: list[Mapping[str, Any]],
    selected: set[str],
    state: dict[str, Any],
    *,
    state_path: Path,
    root: Path,
    results_root: Path,
    cancellation_event: Event | None,
    before_replace: Callable[[Path], None] | None,
) -> tuple[list[dict[str, Any]], str]:
    public: list[dict[str, Any]] = []
    interrupted = False
    for index, item in enumerate(values):
        item_id = str(item["id"])
        if item_id not in selected:
            public.append(
                result_item(
                    item,
                    selected=False,
                    resolution="unselected",
                    status="not-selected",
                    reason="not-selected",
                )
            )
            continue
        if not successful(item):
            state["items"][item_id].update(
                {"status": "incomplete", "reason": "source-not-successful"}
            )
            write(state_path, state)
            public.append(
                result_item(
                    item,
                    selected=True,
                    resolution="incomplete",
                    status="incomplete",
                    reason="source-not-successful",
                )
            )
            continue
        if cancellation_event is not None and cancellation_event.is_set():
            interrupted = True
            reason = "interrupted-before-application"
            state["items"][item_id].update({"status": "incomplete", "reason": reason})
            write(state_path, state)
            public.append(
                result_item(
                    item,
                    selected=True,
                    resolution="incomplete",
                    status="incomplete",
                    reason=reason,
                )
            )
            continue
        entry = state["items"][item_id]
        entry.update(
            {
                "status": "in-progress",
                "attempts": entry.get("attempts", 0) + 1,
                "reason": None,
            }
        )
        write(state_path, state)
        prepared_diff: str | None = None
        try:
            prepared_diff, _, _, _, _ = diff_for(item, root, results_root)
            resolution, reason, diff, readback = apply_one(
                item,
                root=root,
                results_root=results_root,
                cancellation_event=cancellation_event,
                before_replace=before_replace,
            )
        except ApplyInterrupted as error:
            resolution, reason, diff, readback = (
                "incomplete",
                str(error),
                prepared_diff,
                None,
            )
            interrupted = True
        except ApplyError as error:
            reason = str(error)
            resolution = (
                "conflict"
                if reason
                in {"original-changed", "destination-changed", "readback-mismatch"}
                else "incomplete"
            )
            diff, readback = prepared_diff, None
        except (OSError, ValueError) as error:
            resolution = "incomplete"
            reason = f"apply-error: {error}"
            diff, readback = prepared_diff, None
        public_resolution = (
            "applied"
            if resolution == "completed" and reason == "applied"
            else resolution
        )
        entry.update(
            {
                "status": "completed" if resolution == "completed" else resolution,
                "reason": reason,
            }
        )
        write(state_path, state)
        public.append(
            result_item(
                item,
                selected=True,
                resolution=public_resolution,
                status=entry["status"],
                reason=reason,
                diff=diff,
                readback=readback,
            )
        )
        if interrupted:
            for pending in values[index + 1 :]:
                pending_id = str(pending["id"])
                if pending_id in selected:
                    reason = "interrupted"
                    state["items"][pending_id].update(
                        {"status": "incomplete", "reason": reason}
                    )
                    public.append(
                        result_item(
                            pending,
                            selected=True,
                            resolution="incomplete",
                            status="incomplete",
                            reason=reason,
                        )
                    )
            write(state_path, state)
            break
    status = (
        "interrupted"
        if interrupted
        else "conflict"
        if any(item["resolution"] == "conflict" for item in public)
        else "incomplete"
        if any(item["resolution"] == "incomplete" for item in public)
        else "completed"
    )
    return public, status
