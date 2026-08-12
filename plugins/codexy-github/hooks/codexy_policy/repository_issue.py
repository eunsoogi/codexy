"""Owned repository issue-mutation admission."""

from .envelope import Request
from .connector import connector_admitted


def forbidden(request: Request) -> bool:
    return not isinstance(request.tool_input, dict) or not connector_admitted(
        request.tool, request.tool_input, request.cwd,
    )
