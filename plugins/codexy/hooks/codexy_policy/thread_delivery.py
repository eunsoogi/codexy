"""Thread-delivery admission from an authenticated host routing envelope."""

from __future__ import annotations

from typing import cast

from .envelope import Diagnostic, Request

FIELDS = ("model", "thinking")
EXPECTED_CHILD = ("gpt-5.6-luna", "max")
EXPECTED_PARENT = ("gpt-5.6-sol", "medium")
ROUTING_SCHEMA = "codexy.thread-delivery.v2"
ROUTING_FIELDS = frozenset(
    {
        "schema",
        "authenticated",
        "direction",
        "sender_thread_id",
        "target_thread_id",
        "target_model",
        "target_thinking",
    }
)
DIRECTIONS = frozenset({"root_to_child", "child_to_parent"})


def forbidden(request: Request) -> bool | str | Diagnostic:
    session = request.session_id
    if not _non_empty_string(session):
        return Diagnostic("MISSING_IDENTITY", _MISSING_IDENTITY)
    # The host-authenticated sender/target pair is the bounded replacement for
    # native create_thread provenance; this consumer never reconstructs it from
    # transcript records or handoff prose.
    metadata = _routing_metadata(
        request.routing_metadata,
        request.routing_metadata_present,
    )
    if isinstance(metadata, Diagnostic):
        return metadata
    metadata = cast(dict[str, object], metadata)
    direction = cast(str, metadata["direction"])

    data = request.tool_input
    if not isinstance(data, dict):
        return _missing_field_diagnostic(list(FIELDS), direction)
    data = cast(dict[str, object], data)
    missing = _missing_fields(data)
    if missing:
        return _missing_field_diagnostic(missing, direction)

    recipient = data.get("threadId")
    if not _non_empty_string(recipient):
        return Diagnostic("MISSING_RECIPIENT", _MISSING_RECIPIENT)
    if metadata["sender_thread_id"] != session:
        return Diagnostic("MISMATCHED_ROUTING_METADATA", _MISMATCHED_ROUTING)
    if metadata["target_thread_id"] != recipient:
        return Diagnostic("WRONG_RECIPIENT", _WRONG_RECIPIENT)
    if metadata["target_thread_id"] == metadata["sender_thread_id"]:
        return Diagnostic("MISMATCHED_ROUTING_METADATA", _MISMATCHED_ROUTING)

    expected = EXPECTED_PARENT if direction == "child_to_parent" else EXPECTED_CHILD
    if (metadata["target_model"], metadata["target_thinking"]) != expected:
        return Diagnostic("MISMATCHED_ROUTING_METADATA", _MISMATCHED_ROUTING)
    if data["model"] != expected[0]:
        return Diagnostic("UNSUPPORTED_MODEL", _unsupported_model(direction))
    if data["thinking"] != expected[1]:
        return Diagnostic("UNSUPPORTED_THINKING", _unsupported_thinking(direction))
    return False


def _routing_metadata(value: object, present: bool) -> dict[str, object] | Diagnostic:
    if not present:
        return Diagnostic("MISSING_ROUTING_METADATA", _MISSING_ROUTING)
    if not isinstance(value, dict):
        return Diagnostic("MALFORMED_ROUTING_METADATA", _MALFORMED_ROUTING)
    metadata = cast(dict[str, object], value)
    if set(metadata) != ROUTING_FIELDS:
        return Diagnostic("MALFORMED_ROUTING_METADATA", _MALFORMED_ROUTING)
    if metadata["schema"] != ROUTING_SCHEMA or metadata["authenticated"] is not True:
        return Diagnostic("MALFORMED_ROUTING_METADATA", _MALFORMED_ROUTING)
    if (
        not isinstance(metadata["direction"], str)
        or metadata["direction"] not in DIRECTIONS
    ):
        return Diagnostic("MALFORMED_ROUTING_METADATA", _MALFORMED_ROUTING)
    for field in (
        "direction",
        "sender_thread_id",
        "target_thread_id",
        "target_model",
        "target_thinking",
    ):
        if not _non_empty_string(metadata[field]):
            return Diagnostic("MALFORMED_ROUTING_METADATA", _MALFORMED_ROUTING)
    return metadata


def _non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip()) and value == value.strip()


def _missing_fields(data: dict[str, object]) -> list[str]:
    return [field for field in FIELDS if not _non_empty_string(data.get(field))]


def _missing_field_diagnostic(fields: list[str], direction: str | None) -> Diagnostic:
    names = " and ".join(fields)
    code = "MISSING_ROUTE_FIELDS" if len(fields) > 1 else f"MISSING_{fields[0].upper()}"
    route = _route(direction)
    return Diagnostic(
        code,
        f"Missing {names}; {route}. MUST correct the field and MUST retry once.",
    )


def _route(direction: str | None) -> str:
    if direction == "child_to_parent":
        return f"child-to-parent delivery requires {_PARENT_ROUTE}"
    if direction == "root_to_child":
        return f"root-to-child delivery requires {_CHILD_ROUTE}"
    return "thread delivery requires explicit authenticated host routing metadata"


_PARENT_ROUTE = (
    "threadId=<authenticated parent>, model='gpt-5.6-sol', and thinking='medium'"
)
_CHILD_ROUTE = (
    "threadId=<authenticated child>, model='gpt-5.6-luna', and thinking='max'"
)
_MISSING_IDENTITY = (
    "Missing authenticated session_id; MUST NOT retry blindly. "
    "MUST retry only after Codex supplies it."
)
_MISSING_RECIPIENT = (
    "Missing recipient threadId; MUST NOT retry blindly. "
    "MUST provide the recipient from the authenticated host route."
)
_MISSING_ROUTING = (
    "Missing authenticated host routing metadata; MUST NOT retry blindly. "
    "MUST retry only after Codex supplies a bounded codexy_thread_delivery envelope."
)
_MALFORMED_ROUTING = (
    "Malformed or ambiguous authenticated host routing metadata; "
    "MUST NOT retry blindly. MUST obtain a fresh host envelope."
)
_MISMATCHED_ROUTING = (
    "Mismatched authenticated host routing metadata; MUST NOT retry blindly. "
    "MUST stop and obtain a fresh host envelope."
)
_WRONG_RECIPIENT = (
    "Wrong recipient route; MUST set threadId to the authenticated host target "
    f"and MUST use {_PARENT_ROUTE} or {_CHILD_ROUTE}, then MUST correct the route "
    "and MUST retry once. MUST NOT guess a thread ID."
)


def _unsupported_model(direction: str) -> str:
    return (
        f"Unsupported delivery model; {_route(direction)}, then MUST correct the "
        "model and MUST retry once."
    )


def _unsupported_thinking(direction: str) -> str:
    return (
        f"Unsupported delivery thinking; {_route(direction)}, then MUST correct "
        "the thinking and MUST retry once."
    )
