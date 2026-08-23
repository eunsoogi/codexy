"""Require native model and thinking for child-thread creation."""

from __future__ import annotations

from .envelope import Request

REQUIRED_FIELDS = ("model", "thinking")


def forbidden(request: Request) -> bool:
    tool_input = request.tool_input
    if not isinstance(tool_input, dict):
        return True
    return any(
        not isinstance(tool_input.get(field), str) or not tool_input[field].strip()
        for field in REQUIRED_FIELDS
    )
