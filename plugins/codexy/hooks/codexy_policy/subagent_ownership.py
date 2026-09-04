"""Admit only bounded explorers and registered specialists to spawn_agent."""

from __future__ import annotations

import re
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
_DURABLE_OWNERSHIP = re.compile(
    r"(?:\b(?:own|owner|ownership|owned|responsible for)\b.{0,80}\b"
    r"(?:branch|worktree|pull request|pr\b|review[- ]?response|review feedback)\b)"
    r"|(?:\b(?:branch|worktree|pull request|pr\b|review[- ]?response|review feedback)\b"
    r".{0,80}\b(?:owner|ownership|owned|responsible for)\b)"
    r"|(?:\b(?:implement|modify|edit)\b.{0,60}\b(?:in|on)\s+"
    r"(?:the\s+)?(?:reserved|assigned|dedicated|current|child-owned)?\s*"
    r"(?:branch|worktree)\b)"
    r"|(?:\b(?:commit|push)\b.{0,30}\b(?:to|onto|into)\s+"
    r"(?:the\s+)?(?:reserved|assigned|dedicated|current|child-owned)?\s*"
    r"(?:branch|worktree)\b)"
    r"|(?:\b(?:update|handle)\b.{0,35}\b(?:the|this|your|assigned|reserved|dedicated)?\s*"
    r"(?:pull request|pr\b|review[- ]?response|review feedback)\b)"
    r"|(?:\bcreate\s+(?:a|the|this|your)?\s*(?:pull request|pr\b))"
    r"|(?:\b(?:durable|long[- ]running|child[- ]owned|implementation)\b.{0,40}\b"
    r"(?:owner|ownership|lane|context)\b)",
    re.IGNORECASE | re.DOTALL,
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
        if _DURABLE_OWNERSHIP.search(message):
            return "DURABLE_OWNER"
        return False
    if agent_type is None or agent_type == "default" or agent_type == "worker":
        return "GENERIC_IMPLEMENTER"
    return "ROLE"
