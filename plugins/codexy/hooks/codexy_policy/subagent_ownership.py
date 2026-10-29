"""Admit only bounded explorers and registered specialists to spawn_agent."""

from __future__ import annotations

from typing import cast

from .envelope import Diagnostic, Request
from .subagent_ownership_action import delegated_work_reason
from .subagent_ownership_message import durable_owner_requested

# Admission makes these roles valid bounded targets; role-specific behavior remains instruction-level.
# The Watcher MCP is a transport, not admission for the retired native Watcher role.
_SPECIALISTS = frozenset(
    {
        "codexy-architect",
        "codexy-advisor",
        "codexy-auditor",
        "codexy-cartographer",
        "codexy-inspector",
        "codexy-sentinel",
        "codexy-shipwright",
        "codexy-warden",
        "codexy-weaver",
    }
)


def _message_from_input(tool_input: object) -> str | None:
    if not isinstance(tool_input, dict):
        return None
    message = cast("dict[str, object]", tool_input).get("message")
    return message if isinstance(message, str) and message.strip() else None


def _is_bounded_role(agent_type: object) -> bool:
    return isinstance(agent_type, str) and (
        agent_type == "explorer" or agent_type in _SPECIALISTS
    )


def _role_denial(agent_type: object) -> str:
    if agent_type is None or agent_type == "default" or agent_type == "worker":
        return "GENERIC_IMPLEMENTER"
    return "ROLE"


def forbidden(request: Request) -> bool | str:
    """Return the policy reason when a spawn request must be denied."""
    tool_input = request.tool_input
    if not isinstance(tool_input, dict):
        return "ENVELOPE"
    tool_input = cast("dict[str, object]", tool_input)
    message = _message_from_input(tool_input)
    if message is None:
        return "ENVELOPE"
    # Follow-up targets have no authenticated role metadata in the hook envelope.
    # Apply the same message policy to every target rather than trusting a role claim.
    if request.tool.rsplit("__", 1)[-1] in {"send_message", "followup_task"}:
        target = tool_input.get("target")
        if not isinstance(target, str) or not target.strip():
            return "ENVELOPE"
        return "DURABLE_OWNER" if durable_owner_requested(message) else False
    agent_type = tool_input.get("agent_type")
    if not _is_bounded_role(agent_type):
        return _role_denial(agent_type)
    if durable_owner_requested(message):
        return "DURABLE_OWNER"
    return delegated_work_reason(message) or False


_DENIAL_GUIDANCE = (
    "Use a Codex child chat for implementation or execution verification. "
    "This hook does not create or dispatch chats. "
    "구현이나 실행 검증은 Codex 자식 채팅에서 진행하세요. "
    "이 훅은 채팅을 자동 생성하거나 실행하지 않습니다."
)
_GUIDED_REASONS = frozenset(
    {
        "GENERIC_IMPLEMENTER",
        "ROLE",
        "DURABLE_OWNER",
        "IMPLEMENTATION_REQUEST",
        "EXECUTION_VERIFICATION",
    }
)


def forbidden_with_guidance(request: Request) -> bool | str | Diagnostic:
    """Add the child-chat route at the hook boundary without changing classifier callers."""
    result = forbidden(request)
    if isinstance(result, str) and result in _GUIDED_REASONS:
        return Diagnostic(result, _DENIAL_GUIDANCE)
    return result
