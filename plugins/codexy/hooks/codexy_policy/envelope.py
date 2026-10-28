"""Bounded hook-envelope parsing and event-native denial serialization."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Callable, Literal

MAX_INPUT = 1024 * 1024
EVENTS = {"PermissionRequest", "PreToolUse"}


@dataclass(frozen=True)
class Request:
    event: str
    tool: str
    tool_input: object
    cwd: object
    session_id: object
    transcript_path: object
    routing_metadata: object = None
    routing_metadata_present: bool = False


@dataclass(frozen=True)
class Diagnostic:
    code: str
    message: str


@dataclass(frozen=True)
class Advisory:
    event: str
    additional_context: str


@dataclass(frozen=True)
class PolicyResult:
    # Keep policy admission separate from output bytes so advisory context is not
    # misclassified as a denial by timing or other observers.
    output: bytes
    decision: Literal["allow", "deny"]


def _pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
    # Reject duplicate routing or tool keys so an input cannot have conflicting meanings.
    result: dict[str, Any] = {}
    for key, value in items:
        if key in result:
            raise ValueError("duplicate key")
        result[key] = value
    return result


def deny(event: str, diagnostic: str, code: str, message: str | None = None) -> bytes:
    event = event if event in EVENTS else "PreToolUse"
    detail = message or "Codexy policy MUST NOT execute this operation."
    reason = f"{diagnostic}{code}: {detail}"
    if event == "PermissionRequest":
        output = {
            "hookEventName": event,
            "decision": {"behavior": "deny", "message": reason},
        }
    else:
        output = {
            "hookEventName": event,
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    return (
        json.dumps({"hookSpecificOutput": output}, separators=(",", ":")) + "\n"
    ).encode()


def evaluate(
    event: str,
    payload: bytes,
    tools: frozenset[str],
    diagnostic: str,
    forbidden: Callable[[Request], bool | str | Diagnostic | Advisory],
) -> bytes:
    # Preserve the bytes-only API used by the other core hooks.
    return evaluate_result(event, payload, tools, diagnostic, forbidden).output


def evaluate_result(
    event: str,
    payload: bytes,
    tools: frozenset[str],
    diagnostic: str,
    forbidden: Callable[[Request], bool | str | Diagnostic | Advisory],
) -> PolicyResult:
    # Bound the request before decoding; hook input is host-provided and must fail closed when oversized.
    if event not in EVENTS or len(payload) > MAX_INPUT:
        return PolicyResult(deny(event, diagnostic, "ENVELOPE"), "deny")
    try:
        data = json.loads(payload.decode("utf-8", "strict"), object_pairs_hook=_pairs)
    except (UnicodeError, ValueError, json.JSONDecodeError):
        return PolicyResult(deny(event, diagnostic, "ENVELOPE"), "deny")
    if (
        not isinstance(data, dict)
        or data.get("hook_event_name") != event
        or data.get("tool_name") not in tools
    ):
        return PolicyResult(deny(event, diagnostic, "ENVELOPE"), "deny")
    request = Request(
        event,
        data["tool_name"],
        data.get("tool_input"),
        data.get("cwd"),
        data.get("session_id"),
        data.get("transcript_path"),
        data.get("codexy_thread_delivery"),
        "codexy_thread_delivery" in data,
    )
    try:
        blocked = forbidden(request)
    except (OSError, TypeError, ValueError):
        return PolicyResult(deny(event, diagnostic, "RUNTIME"), "deny")
    if isinstance(blocked, Advisory):
        if event != "PreToolUse" or blocked.event != event:
            return PolicyResult(
                deny(event, diagnostic, "INVALID_ADVISORY_EVENT"), "deny"
            )
        output = {
            "hookEventName": event,
            "additionalContext": blocked.additional_context,
        }
        return PolicyResult(
            (
                json.dumps({"hookSpecificOutput": output}, separators=(",", ":")) + "\n"
            ).encode(),
            "allow",
        )
    if isinstance(blocked, Diagnostic):
        return PolicyResult(
            deny(event, diagnostic, blocked.code, blocked.message), "deny"
        )
    if isinstance(blocked, str):
        return PolicyResult(deny(event, diagnostic, blocked), "deny")
    if blocked:
        return PolicyResult(deny(event, diagnostic, "DENIED"), "deny")
    return PolicyResult(b"", "allow")
