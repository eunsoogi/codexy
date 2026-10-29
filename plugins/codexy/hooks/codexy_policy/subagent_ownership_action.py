"""Classify implementation and execution-verification assignments at spawn time."""

from __future__ import annotations

import re

from .subagent_ownership_message import (
    clause_prefix,
    is_negated,
    mask_quoted_data,
)

# Direct mutation verbs stand alone; broader verbs require a source or test object.
_IMPLEMENTATION_TARGET = (
    r"code|source|patch|tests?(?!\s+plan\b)|test\s+(?:cases?|fixtures?|code)|"
    r"module|script|parser|feature|function|files?|implementation|helper|readme|"
    r"docs?|configuration|config|hook|validator|fixture|method|class|component|"
    r"changes?|fix(?:es)?"
)
_IMPLEMENTATION_ACTION = re.compile(
    r"\b(?:implement|modify|edit|change|fix|repair|refactor|patch|commit|push|build)\b"
    + r"|\b(?:update|remove|delete|replace|rewrite|apply|make|write|add|create)\b"
    + r"(?=[^\n;.!?:]{0,55}\b(?:"
    + _IMPLEMENTATION_TARGET
    + r")\b)",
    re.IGNORECASE,
)
_EXECUTION_ACTION = re.compile(
    r"\b(?:run(?:ning|s)?|re[- ]?run(?:ning|s)?|execute|executed|executing|executes|"
    + r"perform|performed|performing|performs|launch|launched|launching|launches|"
    + r"invoke|invoked|invoking|invokes|test)\b",
    re.IGNORECASE,
)
_EXECUTION_TARGET = re.compile(
    r"\b(?:(?:unit|integration|regression|smoke|focused|full|local|package)\s+)?"
    + r"tests?(?!\s+plan\b)\b|\btest\s+suite\b|\bchecks?\b|\blint(?:ing)?\b|"
    + r"\bformat(?:ting)?\b|\bvalidation\b|\bverification\b|\bpackage\s+build\b",
    re.IGNORECASE,
)
# Modals attached to a delegated actor count as assignments; third-person project
# descriptions such as "the parser will implement" remain reviewable content.
_DIRECTIVE_TAIL = re.compile(
    r"\b(?:please|can\s+you|could\s+you|would\s+you|will\s+you|have\s+to|need\s+to)\s*$",
    re.IGNORECASE,
)
_ASSIGNED_MODAL_TAIL = re.compile(
    r"\b(?:you|the\s+(?:assigned\s+)?(?:agent|subagent|child|worker|specialist|"
    + r"reviewer|auditor|helper|implementer))\s+(?:must|should|shall|will)\s*$",
    re.IGNORECASE,
)
# A verification verb followed by "by" assigns the concrete execution that follows.
_VERIFICATION_METHOD_TAIL = re.compile(
    r"^(?:please\s+)?(?:(?:you|the\s+(?:assigned\s+)?(?:agent|subagent|child|worker|"
    + r"specialist|reviewer|auditor|helper|implementer))\s+(?:must|should|shall|will)\s+)?"
    + r"(?:verify|validate|check)\b[^.!?;:]{0,60}\bby\s*$",
    re.IGNORECASE,
)
_ASSIGNMENT_TO = re.compile(
    r"\b(?:need|want|ask|tell|expect(?:ed)?|require(?:d)?|instruct|assign(?:ed)?|"
    + r"task(?:ed)?|intend)\b[^.!?;:]{0,55}\bto\s*$",
    re.IGNORECASE,
)
_INQUIRY_PREFIX = re.compile(
    r"\b(?:what|how|why|where|which|whether)\b[^.!?;:]{0,45}$",
    re.IGNORECASE,
)
_KOREAN_IMPLEMENTATION_ACTION = re.compile(
    r"(?:구현|수정|편집|변경|수리|작성|추가|삭제|제거|반영|적용)"
    + r"(?:하고|해서|해(?:줘|주세요|주십시오|라|요|봐)?|하지\s*(?:말고|마|않)|"
    + r"하(?:고|여|세요|십시오|라|며|면))|"
    + r"고쳐(?:줘|주세요|주십시오|라|요|봐)?",
    re.IGNORECASE,
)
_KOREAN_EXECUTION_ACTION = re.compile(
    r"(?:실행|수행|재실행|돌리)(?:하고|해서|해(?:줘|주세요|주십시오|라|요|봐)?|"
    + r"하지\s*(?:말고|마|않)|하(?:고|여|세요|십시오|라|며|면))|"
    + r"돌려(?:줘|주세요|주십시오|라|요|봐)?|테스트해(?:줘|주세요|주십시오|라|요|봐)?",
    re.IGNORECASE,
)
_KOREAN_EXECUTION_TARGET = re.compile(
    r"(?:테스트|검사|체크|검증|린트|포맷|빌드)",
    re.IGNORECASE,
)
_SENTENCE_BOUNDARY = re.compile(r"[\n.!?;]")


def _sentence_text(message: str, position: int) -> str:
    start = max(
        (match.end() for match in _SENTENCE_BOUNDARY.finditer(message, 0, position)),
        default=0,
    )
    end_match = _SENTENCE_BOUNDARY.search(message, position)
    end = len(message) if end_match is None else end_match.start()
    return message[start:end]


def _instructional_action(message: str, match: re.Match[str]) -> bool:
    """Accept imperative or assigned actions, not read-only questions about them."""
    if is_negated(message, match):
        return False
    prefix = clause_prefix(message, match.start())[-90:]
    if _INQUIRY_PREFIX.search(prefix):
        return False
    tail = re.split(r"\b(?:and|then)\s+", prefix, flags=re.IGNORECASE)[-1].strip(
        " \t\"'`“”"
    )
    if tail.casefold() == "then":
        tail = ""
    return not tail or bool(
        _DIRECTIVE_TAIL.search(tail)
        or _ASSIGNED_MODAL_TAIL.search(tail)
        or _VERIFICATION_METHOD_TAIL.search(tail)
        or _ASSIGNMENT_TO.search(tail)
    )


def delegated_work_reason(message: str) -> str | None:
    """Classify direct source changes and delegated check execution."""
    masked, unmatched_quote = mask_quoted_data(message)
    if unmatched_quote:
        # An unfinished quote leaves action ownership ambiguous, so inspect its raw text.
        if _IMPLEMENTATION_ACTION.search(
            message
        ) or _KOREAN_IMPLEMENTATION_ACTION.search(message):
            return "IMPLEMENTATION_REQUEST"
        if _EXECUTION_ACTION.search(message) or _KOREAN_EXECUTION_ACTION.search(
            message
        ):
            return "EXECUTION_VERIFICATION"

    for match in _IMPLEMENTATION_ACTION.finditer(masked):
        if _instructional_action(masked, match):
            return "IMPLEMENTATION_REQUEST"
    for match in _KOREAN_IMPLEMENTATION_ACTION.finditer(masked):
        if not is_negated(masked, match):
            return "IMPLEMENTATION_REQUEST"

    for match in _EXECUTION_ACTION.finditer(masked):
        if not _instructional_action(masked, match):
            continue
        operation = match.group(0).casefold()
        if operation == "test" or _EXECUTION_TARGET.search(
            _sentence_text(masked, match.start())
        ):
            return "EXECUTION_VERIFICATION"
    for match in _KOREAN_EXECUTION_ACTION.finditer(masked):
        if not is_negated(masked, match) and _KOREAN_EXECUTION_TARGET.search(
            _sentence_text(masked, match.start())
        ):
            return "EXECUTION_VERIFICATION"
    return None
