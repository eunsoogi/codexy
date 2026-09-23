"""Enforce the assigned Worker route for native thread creation."""

from __future__ import annotations

from typing import cast

from .envelope import Diagnostic, Request
from .thread_delivery import FIELDS, WORKER_ROUTE

# Native ordinary-route creation is the assigned Worker route. The hook filename
# remains a compatibility identifier for the installed concern and launcher.
WORKER_MODEL, WORKER_THINKING = WORKER_ROUTE


def forbidden(request: Request) -> bool | Diagnostic:
    tool_input = request.tool_input
    if not isinstance(tool_input, dict):
        return Diagnostic("MISSING_ROUTE_FIELDS", _MISSING_FIELDS)
    data = cast(dict[str, object], tool_input)
    missing = [field for field in FIELDS if not _non_empty_string(data.get(field))]
    if missing:
        return _missing_field_diagnostic(missing)
    if data["model"] != WORKER_MODEL:
        return Diagnostic("UNSUPPORTED_MODEL", _UNSUPPORTED_MODEL)
    if data["thinking"] != WORKER_THINKING:
        return Diagnostic("UNSUPPORTED_THINKING", _UNSUPPORTED_THINKING)
    return False


def _non_empty_string(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip()) and value == value.strip()


def _missing_field_diagnostic(fields: list[str]) -> Diagnostic:
    names = " and ".join(fields)
    code = "MISSING_ROUTE_FIELDS" if len(fields) > 1 else f"MISSING_{fields[0].upper()}"
    return Diagnostic(
        code,
        f"Missing {names}; {_REQUIRED_ROUTE}. MUST correct the field and MUST retry once.",
    )


_REQUIRED_ROUTE = "Worker creation requires model='gpt-6-luna' and thinking='max'"
_MISSING_FIELDS = (
    f"Missing model and thinking; {_REQUIRED_ROUTE}. "
    "MUST correct the fields and MUST retry once."
)
_UNSUPPORTED_MODEL = (
    f"Unsupported Worker creation model; {_REQUIRED_ROUTE}. "
    "MUST NOT substitute another model or silently fall back."
)
_UNSUPPORTED_THINKING = (
    f"Unsupported Worker creation thinking; {_REQUIRED_ROUTE}. "
    "MUST NOT substitute another reasoning setting or silently fall back."
)
