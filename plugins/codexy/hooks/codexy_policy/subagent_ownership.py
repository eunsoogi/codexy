"""Admit only bounded explorers and registered specialists to spawn_agent."""

from __future__ import annotations

from typing import cast

from .envelope import Request

_SPECIALISTS = frozenset(
    {
        "codexy-architect",
        "codexy-auditor",
        "codexy-cartographer",
        "codexy-inspector",
        "codexy-sentinel",
        "codexy-shipwright",
        "codexy-warden",
        "codexy-weaver",
    }
)


def forbidden(request: Request) -> bool | str:
    tool_input = request.tool_input
    if not isinstance(tool_input, dict):
        return "ENVELOPE"
    tool_input = cast("dict[str, object]", tool_input)
    message = tool_input.get("message")
    if not isinstance(message, str) or not message.strip():
        return "ENVELOPE"
    agent_type = tool_input.get("agent_type")
    if isinstance(agent_type, str) and (
        agent_type == "explorer" or agent_type in _SPECIALISTS
    ):
        return False
    if agent_type is None or agent_type == "default" or agent_type == "worker":
        return "GENERIC_IMPLEMENTER"
    return "ROLE"
