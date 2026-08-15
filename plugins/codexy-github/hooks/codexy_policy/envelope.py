"""Bounded hook-envelope parsing and event-native denial serialization."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Callable

MAX_INPUT = 1024 * 1024
EVENTS = {"PermissionRequest", "PreToolUse"}


@dataclass(frozen=True)
class Request:
    event: str
    tool: str
    tool_input: object
    cwd: object


def _pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in items:
        if key in result:
            raise ValueError("duplicate key")
        result[key] = value
    return result


def deny(event: str, diagnostic: str, code: str) -> bytes:
    event = event if event in EVENTS else "PreToolUse"
    reason = f"{diagnostic}{code}: Codexy policy MUST NOT execute this operation."
    output = (
        {"hookEventName": event, "decision": {"behavior": "deny", "message": reason}}
        if event == "PermissionRequest"
        else {
            "hookEventName": event,
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    )
    return (
        json.dumps({"hookSpecificOutput": output}, separators=(",", ":")) + "\n"
    ).encode()


def evaluate(
    event: str,
    payload: bytes,
    tools: frozenset[str],
    diagnostic: str,
    forbidden: Callable[[Request], bool | str],
) -> bytes:
    if event not in EVENTS or len(payload) > MAX_INPUT:
        return deny(event, diagnostic, "ENVELOPE")
    try:
        data = json.loads(payload.decode("utf-8", "strict"), object_pairs_hook=_pairs)
    except (UnicodeError, ValueError, json.JSONDecodeError):
        return deny(event, diagnostic, "ENVELOPE")
    if (
        not isinstance(data, dict)
        or data.get("hook_event_name") != event
        or data.get("tool_name") not in tools
    ):
        return deny(event, diagnostic, "ENVELOPE")
    request = Request(event, data["tool_name"], data.get("tool_input"), data.get("cwd"))
    try:
        blocked = forbidden(request)
    except (OSError, TypeError, ValueError):
        return deny(event, diagnostic, "RUNTIME")
    if isinstance(blocked, str):
        return deny(event, diagnostic, blocked)
    return deny(event, diagnostic, "DENIED") if blocked else b""
