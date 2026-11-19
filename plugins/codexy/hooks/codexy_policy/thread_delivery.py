"""Validate thread-delivery request fields and the selected model pair."""

from __future__ import annotations

import sqlite3
from dataclasses import dataclass
from typing import Callable, Literal, Mapping, cast

from .envelope import Diagnostic, Request

WORKER_ROUTE = ("gpt-6.1-terra", "medium")
HARD_WORKER_ROUTE = ("gpt-6.2-sol", "medium")
# Creation still requires an explicit pair; delivery also requires the recipient.
FIELDS = ("model", "thinking")
WORKER_CREATION_ROUTES = frozenset({WORKER_ROUTE, HARD_WORKER_ROUTE})
ORCHESTRATOR_ROUTE = ("gpt-6.2-sol", "low")
ALLOWED_PAIRS = (
    ORCHESTRATOR_ROUTE,
    WORKER_ROUTE,
    HARD_WORKER_ROUTE,
)
_ALLOWED_MODELS = frozenset(model for model, _thinking in ALLOWED_PAIRS)
_REQUIRED_FIELDS = ("threadId", "model", "thinking")
_SUPPORTED_PAIRS = ", ".join(f"{model}/{thinking}" for model, thinking in ALLOWED_PAIRS)


@dataclass(frozen=True)
class RecipientAssignment:
    """Result supplied by a trusted supervision lookup, never by tool input."""

    status: Literal["assigned", "unavailable", "unrelated"]
    model: str | None = None
    thinking: str | None = None


def resolve_supervised_assignment(
    parent: str,
    sender: str,
    recipient: str,
    lookup: Callable[[str, str], Mapping[str, object] | None],
) -> RecipientAssignment:
    """Consume a parent-scoped lookup after the caller authenticates the identities.

    A missing record cannot distinguish an unrelated chat from a lost assignment;
    the caller must establish unrelated traffic separately before using that status.
    """
    unavailable = RecipientAssignment("unavailable")
    if not all(_non_empty_string(value) for value in (parent, sender, recipient)):
        return unavailable
    if sender == parent and recipient != parent:
        child = recipient
    elif recipient == parent and sender != parent:
        child = sender
    else:
        return unavailable
    try:
        record = lookup(parent, child)
    except (OSError, ValueError, sqlite3.Error):
        return unavailable
    if (
        not isinstance(record, Mapping)
        or record.get("parent") != parent
        or record.get("child") != child
        or record.get("state") != "active"
    ):
        return unavailable
    model, thinking = record.get("model"), record.get("thinking")
    if (
        not isinstance(model, str)
        or not isinstance(thinking, str)
        or (model, thinking) not in WORKER_CREATION_ROUTES
    ):
        return unavailable
    if recipient == parent:
        return RecipientAssignment("assigned", *ORCHESTRATOR_ROUTE)
    return RecipientAssignment("assigned", model, thinking)


def recipient_forbidden(
    request: Request, assignment: RecipientAssignment
) -> bool | str | Diagnostic:
    """Apply the recipient decision once an authenticated lookup is available."""
    data = request.tool_input
    if not isinstance(data, dict) or not _non_empty_string(data.get("threadId")):
        return _missing_field_diagnostic(["threadId"])
    if assignment.status == "unrelated":
        return forbidden(request)
    pair = (assignment.model, assignment.thinking)
    # Unknown or corrupt supervised state must not silently become an unrelated chat.
    if assignment.status != "assigned" or pair not in ALLOWED_PAIRS:
        return Diagnostic(
            "ASSIGNMENT_UNAVAILABLE",
            "The recipient's supervision assignment is unavailable or invalid. "
            "Recover the trusted assignment before retrying; do not guess the model or thinking.",
        )
    if (data.get("model"), data.get("thinking")) != pair:
        return Diagnostic(
            "RECIPIENT_ROUTE_MISMATCH",
            f"Use the recipient's assigned model/thinking pair {pair[0]}/{pair[1]}; "
            "keep the same recipient and retry only with that pair.",
        )
    return False


def forbidden(
    request: Request, assignment: RecipientAssignment | None = None
) -> bool | str | Diagnostic:
    """Validate fields, optionally binding them to a trusted recipient lookup."""
    if assignment is not None:
        return recipient_forbidden(request, assignment)
    data = request.tool_input
    if not isinstance(data, dict):
        return _missing_field_diagnostic(list(_REQUIRED_FIELDS))

    fields = cast(dict[str, object], data)
    missing = [
        field for field in _REQUIRED_FIELDS if not _non_empty_string(fields.get(field))
    ]
    if missing:
        return _missing_field_diagnostic(missing)

    model = cast(str, fields["model"])
    thinking = cast(str, fields["thinking"])
    if model not in _ALLOWED_MODELS:
        return Diagnostic("UNSUPPORTED_MODEL", _unsupported_model())
    if (model, thinking) not in ALLOWED_PAIRS:
        return Diagnostic("UNSUPPORTED_THINKING", _unsupported_thinking())
    return False


def _non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip()) and value == value.strip()


def _missing_field_diagnostic(fields: list[str]) -> Diagnostic:
    if fields == ["threadId"]:
        code = "MISSING_RECIPIENT"
    elif fields == ["model"]:
        code = "MISSING_MODEL"
    elif fields == ["thinking"]:
        code = "MISSING_THINKING"
    elif set(fields) == {"model", "thinking"}:
        code = "MISSING_ROUTE_FIELDS"
    else:
        code = "MISSING_DELIVERY_FIELDS"

    names = _field_names(fields)
    return Diagnostic(
        code,
        f"Missing or empty {names}; required fields are threadId, model, and thinking. "
        + f"Allowed model/thinking pairs: {_SUPPORTED_PAIRS}. "
        + _missing_field_correction(fields),
    )


def _field_names(fields: list[str]) -> str:
    if len(fields) == 1:
        return fields[0]
    if len(fields) == 2:
        return f"{fields[0]} and {fields[1]}"
    return ", ".join(fields[:-1]) + f", and {fields[-1]}"


# Recipient recovery must use the assignment; pair-only edits cannot recreate
# a missing thread ID.
def _missing_field_correction(fields: list[str]) -> str:
    if "threadId" not in fields:
        return _CORRECTION

    names = _field_names(fields)
    field_word = "field" if len(fields) == 1 else "fields"
    return (
        "Use the known recipient's threadId from the assignment; do not guess. "
        "Keep the recipient's assigned pair; "
        f"correct only the missing {field_word} {names}, then retry only the corrected request."
    )


_CORRECTION = (
    "Keep the known recipient and orchestration-assigned pair; correct only the "
    "editable model/thinking fields, then retry only the corrected request."
)


def _unsupported_model() -> str:
    return (
        f"Unsupported model field. Allowed model/thinking pairs: {_SUPPORTED_PAIRS}. "
        f"{_CORRECTION}"
    )


def _unsupported_thinking() -> str:
    return (
        "Unsupported thinking field for the selected model. "
        f"Allowed model/thinking pairs: {_SUPPORTED_PAIRS}. {_CORRECTION}"
    )
