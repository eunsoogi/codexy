"""Interpret documented cleanup tool inputs without inferring eligibility."""

from __future__ import annotations

import re
from typing import Any

ARCHIVE_TOOLS = frozenset(
    {"codex_app__set_thread_archived", "mcp__codex_app__set_thread_archived"}
)
LIST_TOOLS = frozenset(
    {"codex_app__list_archived_threads", "mcp__codex_app__list_archived_threads"}
)
_THREAD_ID = re.compile(r"[A-Za-z0-9_-]{1,128}\Z")


def observation(payload: dict[str, Any]) -> tuple[str, str | None] | None:
    """Classify an observed invocation, not the underlying archive outcome."""
    if payload.get("hook_event_name") != "PostToolUse":
        return None
    tool = payload.get("tool_name")
    if tool in LIST_TOOLS:
        # Even an unsuccessful or incomplete list attempt ends automatic prompting;
        # the parent must interpret its result and decide how to proceed.
        return ("list_attempt", None)
    if tool not in ARCHIVE_TOOLS:
        return None
    data = payload.get("tool_input")
    response = payload.get("tool_response")
    if not isinstance(data, dict) or data.get("archived") is not True:
        return None
    thread_id = data.get("threadId")
    if not isinstance(thread_id, str) or not _THREAD_ID.fullmatch(thread_id):
        return None
    # MCP's optional isError indicates failure when true. A non-error envelope
    # establishes only that the invocation returned, not that archival succeeded.
    if not isinstance(response, dict) or not isinstance(response.get("content"), list):
        return None
    if response.get("isError", False) is not False:
        return None
    return ("archive_returned", thread_id)


def continuation(thread_id: str) -> dict[str, str]:
    """Request a harmless readback without claiming identity or eligibility."""
    return {
        "decision": "block",
        "reason": (
            f"The archive request for {thread_id} returned, but its outcome is "
            "unconfirmed. Use list_archived_threads to check the exact identity. "
            "If confirmation is unavailable, preserve resources and report the gap. "
            "Do not infer acceptance, eligibility, or branch deletion authority."
        ),
    }
