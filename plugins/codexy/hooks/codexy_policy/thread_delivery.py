"""Authenticated recipient-route admission for Codex task delivery."""

import json
from typing import cast

from .envelope import Diagnostic, Request
from .thread_delivery_support import (
    authenticated_parent,
    duplicate_delivery,
    records as read_records,
)

FIELDS = ("model", "thinking")
EXPECTED_CHILD = ("gpt-5.6-luna", "max")
EXPECTED_PARENT = ("gpt-5.6-sol", "medium")


def forbidden(request: Request) -> bool | str | Diagnostic:
    session = request.session_id
    transcript = request.transcript_path
    if session is None and transcript is None:
        return Diagnostic("MISSING_IDENTITY", _MISSING_IDENTITY)
    if (
        not isinstance(session, str)
        or not session.strip()
        or not isinstance(transcript, str)
        or not transcript.strip()
    ):
        return Diagnostic("MISSING_IDENTITY", _MISSING_IDENTITY)
    try:
        records = read_records(transcript)
        parent = authenticated_parent(records, session)
    except (OSError, TypeError, UnicodeError, ValueError, json.JSONDecodeError):
        return Diagnostic("UNTRUSTED_CONTEXT", _UNTRUSTED_CONTEXT)
    data = request.tool_input
    if not isinstance(data, dict):
        return _missing_field_diagnostic(list(FIELDS), parent is not None)
    data = cast(dict[str, object], data)
    missing = _missing_fields(data)
    if missing:
        return _missing_field_diagnostic(missing, parent is not None)

    recipient = data.get("threadId")
    if not isinstance(recipient, str) or not recipient.strip():
        return _missing_recipient_diagnostic(parent is not None)

    route = (data["model"], data["thinking"])
    if parent is None:
        # A root task may deliver only to a distinct child using the child's
        # explicitly selected model/thinking pair. It must never route to itself.
        if recipient == session:
            return Diagnostic("WRONG_RECIPIENT", _ROOT_WRONG_RECIPIENT)
        if route != EXPECTED_CHILD:
            if data["model"] != EXPECTED_CHILD[0]:
                return Diagnostic("UNSUPPORTED_MODEL", _UNSUPPORTED_CHILD_MODEL)
            return Diagnostic("UNSUPPORTED_THINKING", _UNSUPPORTED_CHILD_THINKING)
    else:
        # A delegated child may report only to its authenticated parent with
        # the parent route settings. Never infer or accept another recipient.
        if recipient != parent:
            return Diagnostic("WRONG_RECIPIENT", _WRONG_RECIPIENT)
        if data["model"] != EXPECTED_PARENT[0]:
            return Diagnostic("UNSUPPORTED_MODEL", _UNSUPPORTED_MODEL)
        if data["thinking"] != EXPECTED_PARENT[1]:
            return Diagnostic("UNSUPPORTED_THINKING", _UNSUPPORTED_THINKING)
    return duplicate_delivery(records, data)


_ROUTE = "threadId=<authenticated parent>, model='gpt-5.6-sol', and thinking='medium'"
_ROOT_ROUTE = "non-empty model and thinking values selected for the child"
_MISSING_IDENTITY = (
    "Missing authenticated session_id or transcript_path; MUST NOT retry blindly. "
    "MUST retry only after Codex supplies both values."
)
_UNTRUSTED_CONTEXT = (
    "Untrusted or malformed delegation context; MUST NOT retry blindly. "
    "MUST stop and obtain a fresh authenticated child context."
)
_WRONG_RECIPIENT = (
    "Wrong recipient route; MUST set threadId to the authenticated parent from "
    f"the delegation context and MUST use {_ROUTE} for child-to-parent delivery, "
    "then MUST correct the route and MUST retry once. MUST NOT guess a parent ID."
)
_ROOT_WRONG_RECIPIENT = (
    "Wrong recipient route; root-to-child delivery MUST target a distinct child "
    f"and MUST use model='{EXPECTED_CHILD[0]}' and thinking='{EXPECTED_CHILD[1]}', "
    "then MUST correct the route and MUST retry once."
)
_UNSUPPORTED_MODEL = (
    f"Unsupported delivery model; MUST use {_ROUTE} for child-to-parent delivery, "
    "then MUST correct the model and MUST retry once."
)
_UNSUPPORTED_THINKING = (
    f"Unsupported delivery thinking; MUST use {_ROUTE} for child-to-parent "
    "delivery, then MUST correct the thinking and MUST retry once."
)
_UNSUPPORTED_CHILD_MODEL = (
    f"Unsupported delivery model; root-to-child delivery requires {_ROOT_ROUTE}, "
    "then MUST correct the model and MUST retry once."
)
_UNSUPPORTED_CHILD_THINKING = (
    f"Unsupported delivery thinking; root-to-child delivery requires {_ROOT_ROUTE}, "
    "then MUST correct the thinking and MUST retry once."
)
_MISSING_RECIPIENT = (
    "Missing threadId; delivery MUST target the authenticated route and MUST "
    "correct the field before retrying once."
)
_MISSING_ROOT_RECIPIENT = (
    "Missing threadId; root-to-child delivery MUST target a distinct child and "
    "MUST correct the field before retrying once."
)


def _missing_fields(data: dict[str, object]) -> list[str]:
    missing: list[str] = []
    for field in FIELDS:
        value = data.get(field)
        if not isinstance(value, str) or not value.strip():
            missing.append(field)
    return missing


def _missing_field_diagnostic(fields: list[str], child_to_parent: bool) -> Diagnostic:
    names = " and ".join(fields)
    code = "MISSING_ROUTE_FIELDS" if len(fields) > 1 else f"MISSING_{fields[0].upper()}"
    if not child_to_parent:
        return Diagnostic(
            code,
            f"Missing {names}; root-to-child delivery requires {_ROOT_ROUTE}. "
            "MUST provide the missing value, then MUST correct the route and MUST retry once.",
        )
    return Diagnostic(
        code,
        f"Missing {names}; child-to-parent delivery requires {_ROUTE}. "
        "MUST correct the field and MUST retry once.",
    )


def _missing_recipient_diagnostic(child_to_parent: bool) -> Diagnostic:
    return Diagnostic(
        "MISSING_RECIPIENT",
        _MISSING_RECIPIENT if child_to_parent else _MISSING_ROOT_RECIPIENT,
    )
