"""Reconcile a correlated creation response without guessing missing outcomes."""

from __future__ import annotations

import json
from typing import Any

from .supervision_reservations import ReservationError, Reservations

CREATE_TOOLS = frozenset({"codex_app__create_thread", "mcp__codex_app__create_thread"})


def _unique(items: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in items:
        if key in result:
            raise ValueError("duplicate field")
        result[key] = value
    return result


def _result(response: object) -> dict[str, Any] | None:
    if not isinstance(response, dict) or response.get("isError", False) is not False:
        return None
    data = response.get("structuredContent")
    content = response.get("content")
    # Interpret only an unambiguous MCP result; an error or unfamiliar shape
    # leaves the reservation in place until the caller reconciles it.
    if data is None and isinstance(content, list) and len(content) == 1:
        item = content[0]
        if not isinstance(item, dict) or item.get("type") != "text":
            return None
        try:
            data = json.loads(item.get("text", ""), object_pairs_hook=_unique)
        except (ValueError, TypeError):
            return None
    return data if isinstance(data, dict) else None


def observe_create(store: Reservations, event: dict[str, Any]) -> str:
    """Reconcile a result against the same session and pre-reserved call ID.

    The event producer is responsible for authentication; unknown or conflicting
    observations do not free a slot and are reported as uncertain.
    """
    if (
        event.get("hook_event_name") != "PostToolUse"
        or event.get("tool_name") not in CREATE_TOOLS
    ):
        return "ignored"
    parent, request = event.get("session_id"), event.get("tool_use_id")
    data = _result(event.get("tool_response"))
    if not isinstance(parent, str) or not isinstance(request, str) or data is None:
        return "uncertain"
    child, pending = data.get("threadId"), data.get("clientThreadId")
    if bool(child) == bool(pending):
        return "uncertain"
    try:
        if isinstance(child, str) and not pending:
            store.ready(parent, request, child)
            return "ready"
        if isinstance(pending, str) and not child:
            store.setup(parent, request, pending)
            return "pending"
    except ReservationError:
        return "uncertain"
    return "uncertain"
