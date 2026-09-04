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
    r"(?:"
    r"\b(?:own(?:s|ed|ing|ership)?|owner|ownership|owned|responsible\s+for|"
    r"take\s+ownership(?:\s+of)?|manage|lead)\b[^\n,;.!?:]{0,80}\b"
    r"(?:branch|worktree|pull\s+request|pr\b|review[- ]?response|review feedback)\b"
    r"|\b(?:branch|worktree|pull\s+request|pr\b|review[- ]?response|review feedback)\b"
    r"[^\n,;.!?:]{0,80}\b(?:own(?:s|ed|ing|ership)?|owner|ownership|owned|responsible\s+for|"
    r"take\s+ownership(?:\s+of)?|manage|lead)\b"
    r"|\b(?:implement|modify|edit|commit|push|update|handle|address|resolve|fix|apply|complete)\b"
    r"[^\n;.!?:]{0,70}\b(?:in|on|to|onto|into)\s+(?:the\s+)?"
    r"(?:reserved|assigned|dedicated|current|child-owned)?\s*(?:branch|worktree)\b"
    r"|\b(?:reserved|assigned|dedicated|current|child-owned)\s+(?:branch|worktree)\b"
    r"[^\n;.!?:]{0,80}\b(?:implement|modify|edit|commit|push|update|handle|address|resolve|fix|apply|complete)\b"
    r"|\b(?:address|resolve|complete|fix|apply|handle|implement|modify|edit|commit|push|update)\b"
    r"[^\n;.!?]{0,70}\b(?:review[- ]?response|review feedback)\b"
    r"|\b(?:review[- ]?response|review feedback)\b[^\n;.!?]{0,70}\b"
    r"(?:address|resolve|complete|fix|apply|handle|implement|modify|edit|commit|push|update)\b"
    r"|\b(?:create|open|submit|file)\s+(?:a|the|this|your)?\s*(?:pull\s+request|pr\b)"
    r"|\b(?:durable|long[- ]running|child[- ]owned|implementation)\b.{0,40}\b"
    r"(?:owner|ownership|lane|context)\b"
    r")",
    re.IGNORECASE,
)
_DURABLE_OWNERSHIP_KO = re.compile(
    r"(?:"
    r"(?:소유(?:권|자)?|담당|책임|전담|맡\w*|관리)[^\n,;.!?:]{0,50}"
    r"(?:브랜치|워크트리|풀\s*리퀘스트|PR|리뷰\s*(?:응답|피드백|의견))"
    r"|(?:브랜치|워크트리|풀\s*리퀘스트|PR|리뷰\s*(?:응답|피드백|의견))[^\n,;.!?:]{0,50}"
    r"(?:소유(?:권|자)?|담당|책임|전담|맡\w*|관리)"
    r"|(?:할당된|예약된|전용|현재|이|해당)\s*(?:브랜치|워크트리)\s*"
    r"(?:에서|로|에)[^\n;.!?:]{0,60}(?:구현|수정|편집|커밋|푸시|생성|열|작성|처리|해결|반영|적용)"
    r"|(?:구현|수정|편집|커밋|푸시|생성|열|작성|처리|해결|반영|적용)[^\n;.!?:]{0,60}"
    r"(?:할당된|예약된|전용|현재|이|해당)\s*(?:브랜치|워크트리)"
    r"|(?:리뷰\s*(?:응답|피드백|의견))[^\n;.!?:]{0,60}"
    r"(?:처리|해결|수정|반영|적용|완료|구현|커밋|푸시|업데이트|담당|맡\w*)"
    r"|(?:처리|해결|수정|반영|적용|완료|구현|커밋|푸시|업데이트|담당|맡\w*)[^\n;.!?:]{0,60}"
    r"리뷰\s*(?:응답|피드백|의견)"
    r")",
    re.IGNORECASE,
)
_NEGATION = re.compile(
    r"(?:\b(?:do\s+not|don't|dont|never|not|no|without)\b|"
    r"(?:금지|하지\s*말\w*|말고|않\w*|아니\w*|없\w*))",
    re.IGNORECASE,
)


def _is_negated(message: str, match: re.Match[str]) -> bool:
    clause_start = (
        max(
            message.rfind("\n", 0, match.start()),
            message.rfind(";", 0, match.start()),
            message.rfind(".", 0, match.start()),
            message.rfind("!", 0, match.start()),
            message.rfind("?", 0, match.start()),
            message.rfind(",", 0, match.start()),
            message.rfind(":", 0, match.start()),
        )
        + 1
    )
    prefix = message[max(clause_start, match.start() - 80) : match.start()]
    if _NEGATION.search(prefix) or _NEGATION.search(match.group(0)):
        return True
    suffix = message[match.end() : match.end() + 32]
    return bool(re.search(r"(?:하지\s*말\w*|말고|않\w*)", suffix))


def _durable_owner_requested(message: str) -> bool:
    for matcher in (_DURABLE_OWNERSHIP, _DURABLE_OWNERSHIP_KO):
        for match in matcher.finditer(message):
            if not _is_negated(message, match):
                return True
    return False


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
        if _durable_owner_requested(message):
            return "DURABLE_OWNER"
        return False
    if agent_type is None or agent_type == "default" or agent_type == "worker":
        return "GENERIC_IMPLEMENTER"
    return "ROLE"
