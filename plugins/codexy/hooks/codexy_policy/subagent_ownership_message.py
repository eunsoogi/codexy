"""Classify durable ownership requests in bounded natural-language messages."""

from __future__ import annotations

import re

_CLAUSE = "\n,;.!?:"
_HINTS = (
    "own|owner|ownership|responsible|manage|lead|branch|worktree|pull request|pr|"
    "review|implement|modify|edit|commit|push|update|handle|address|resolve|fix|"
    "apply|complete|build|write|durable|long-running|child-owned|implementation|"
    "소유|담당|책임|전담|맡|관리|브랜치|워크트리|리퀘스트|리뷰|구현|수정|편집|"
    "커밋|푸시|생성|열|작성|처리|해결|반영|적용"
).split("|")
_DURABLE_OWNERSHIP = re.compile(
    r"(?:"
    + r"\b(?:own(?:s|ed|ing|ership)?|owner|ownership|owned|"
    + r"responsible\s+for(?!\s+review(?:ing)?\b)|"
    + r"take\s+ownership(?:\s+of)?|manage|lead)\b[^\n,;.!?:]{0,80}\b"
    + r"(?:branch|worktree|pull\s+request|pr\b|review[- ]?response|review feedback)\b"
    + r"|\b(?:branch|worktree|pull\s+request|pr\b|review[- ]?response|review feedback)\b"
    + r"[^\n,;.!?:]{0,80}\b(?:own(?:s|ed|ing|ership)?|owner|ownership|owned|"
    + r"responsible\s+for(?!\s+review(?:ing)?\b)|"
    + r"take\s+ownership(?:\s+of)?|manage|lead)\b"
    + r"|\b(?:implement|modify|edit|commit|push|update|handle|address|resolve|fix|apply|complete)\b"
    + r"[^\n;.!?:]{0,70}\b(?:in|on|to|onto|into)\s+(?:the\s+)?"
    + r"(?:reserved|assigned|dedicated|current|child-owned)?\s*(?:branch|worktree)\b"
    + r"|\b(?:reserved|assigned|dedicated|current|child-owned)\s+(?:branch|worktree)\b"
    + r"[^\n;.!?:]{0,80}\b(?:implement|modify|edit|commit|push|update|handle|address|resolve|fix|apply|complete)\b"
    + r"|\b(?:in|on|to|onto|into)\s+(?:the\s+)?(?:branch|worktree)\b"
    + r"[^\n;.!?]{0,90}\b(?:implement|modify|edit|commit|push|update|handle|address|resolve|fix|apply|complete)\b"
    + r"|\b(?:address|resolve|complete|fix|apply|handle|implement|modify|edit|commit|push|update)\b"
    + r"[^\n;.!?]{0,70}\b(?:review[- ]?response|review feedback)\b"
    + r"|\b(?:review[- ]?response|review feedback)\b[^\n;.!?]{0,70}\b"
    + r"(?:address|resolve|complete|fix|apply|handle|implement|modify|edit|commit|push|update)\b"
    + r"|\b(?:create|open|submit|file)\s+(?:a|the|this|your)?\s*(?:pull\s+request|pr\b)"
    + r"|\b(?:durable|long[- ]running|child[- ]owned|implementation)\b.{0,40}\b"
    + r"(?:owner|ownership|lane|context)\b"
    + r")",
    re.IGNORECASE,
)
_DURABLE_BUILD_WRITE = re.compile(
    r"(?im)(?:"
    + r"(?:^|[.!?;\n]\s*|,\s*(?:and|then|please)\s+)"
    + r"(?:please\s+|must\s+|should\s+)?(?:build|write)\b"
    + r"[^\n;.!?:]{0,70}\b(?:in|on|to|onto|into)\s+(?:the\s+)?"
    + r"(?:reserved|assigned|dedicated|current|child-owned)?\s*(?:branch|worktree)\b"
    + r"|(?:^|[.!?;\n]\s*)(?:in|on|to|onto|into)\s+(?:the\s+)?"
    + r"(?:reserved|assigned|dedicated|current|child-owned)?\s*(?:branch|worktree)\b"
    + r"[^\n;.!?:]{0,90}\b(?:build|write)\b"
    + r")"
)
_DURABLE_OWNERSHIP_KO = re.compile(
    r"(?:"
    + r"(?:소유(?:권|자)?|담당|책임|전담|맡\w*|관리)[^\n,;.!?:]{0,50}"
    + r"(?:브랜치|워크트리|풀\s*리퀘스트|PR|리뷰\s*(?:응답|피드백|의견))"
    + r"|(?:브랜치|워크트리|풀\s*리퀘스트|PR|리뷰\s*(?:응답|피드백|의견))[^\n,;.!?:]{0,50}"
    + r"(?:소유(?:권|자)?|담당|책임|전담|맡\w*|관리)"
    + r"|(?:할당된|예약된|전용|현재|이|해당)\s*(?:브랜치|워크트리)\s*"
    + r"(?:에서|로|에)[^\n;.!?:]{0,60}(?:구현|수정|편집|커밋|푸시|생성|열|작성|처리|해결|반영|적용)"
    + r"|(?:구현|수정|편집|커밋|푸시|생성|열|작성|처리|해결|반영|적용)[^\n;.!?:]{0,60}"
    + r"(?:할당된|예약된|전용|현재|이|해당)\s*(?:브랜치|워크트리)"
    + r"|(?:리뷰\s*(?:응답|피드백|의견))[^\n;.!?:]{0,60}"
    + r"(?:처리|해결|수정|반영|적용|완료|구현|커밋|푸시|업데이트|담당|맡\w*)"
    + r"|(?:처리|해결|수정|반영|적용|완료|구현|커밋|푸시|업데이트|담당|맡\w*)[^\n;.!?:]{0,60}"
    + r"리뷰\s*(?:응답|피드백|의견)"
    + r")",
    re.IGNORECASE,
)
_DURABLE_RULES = (
    _DURABLE_OWNERSHIP,
    _DURABLE_OWNERSHIP_KO,
    _DURABLE_BUILD_WRITE,
)
_NEGATION_RULES = (
    re.compile(
        r"(?:\b(?:do\s+not|don't|dont|never|not|no)\b|"
        + r"(?:금지|하지\s*말\w*|말고|않\w*|아니\w*|없\w*|지\s*마))",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?:\b(?:do\s+not|don't|dont|never|not|no)\b"
        + r"(?:\s+[\w'-]+){0,4}\s*$|(?:금지|하지\s*말\w*|말고|않\w*|아니\w*|없\w*)\s*$)",
        re.IGNORECASE,
    ),
    re.compile(
        r"\bwithout\s+(?:own(?:s|ed|ing|ership)?|ownership|"
        + r"being\s+responsible|taking\s+ownership)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?:하지\s*말\w*|말고|않\w*|지\s*마|^\s*마(?:세요|요|십시오)?(?:\W|$))",
        re.IGNORECASE,
    ),
)
_POSITIVE_NEGATION = re.compile(
    r"\b(?:do\s+not|don't|dont)\s+(?:not|"
    + r"(?:say|tell|instruct)\s+not\s+to|(?:hesitate|fail)\s+to)\s*$|"
    + r"(?:주저|망설|머뭇)하?지\s*말고\s*$|"
    + r"(?:하지\s*말라고|하지\s*말라는)\s*하지\s*말고\s*$",
    re.IGNORECASE,
)
_QUOTES = {'"': '"', "‘": "’", "“": "”", "「": "」", "『": "』", "‹": "›", "《": "》"}
_RELAY = re.compile(
    r"(?:\b(?:follow|obey|execute|apply|use)\s+(?:this|the|following)\s+"
    + r"(?:instruction|instructions|direction|directions|prompt)\b|"
    + r"\b(?:follow|obey|execute|apply|use)\s+(?:it|that)\b|"
    + r"(?:다음|아래)\s*(?:지시|문구|명령)(?:를|을)?\s*"
    + r"(?:그대로\s*)?(?:따라|실행|적용|수행)|"
    + r"(?:그대로\s*따라|(?:그|해당|이)\s*(?:지시|문구|명령)(?:를|을)?\s*따라))",
    re.IGNORECASE,
)
_RELAY_GAP = 120


def _escaped(message: str, index: int) -> bool:
    count = 0
    for index in range(index - 1, -1, -1):
        if message[index] != "\\":
            break
        count += 1
    return count % 2 == 1


def _apostrophe(message: str, index: int) -> bool:
    return (
        index > 0
        and index + 1 < len(message)
        and message[index - 1].isalnum()
        and message[index + 1].isalnum()
    )


def _active_relay(message: str, opening: int, closing: int | None) -> bool:
    start = max(0, opening - (2 * _RELAY_GAP))
    end = opening if closing is None else min(len(message), closing + _RELAY_GAP + 1)
    for relay in _RELAY.finditer(message, start, end):
        before = relay.end() <= opening and opening - relay.end() <= _RELAY_GAP
        after = (
            closing is not None
            and relay.start() >= closing
            and relay.start() - closing <= _RELAY_GAP
        )
        if (before or after) and not _is_negated(message, relay):
            return True
    return False


def _mask_quoted_data(message: str) -> tuple[str, bool]:
    masked = list(message)
    opening: int | None = None
    closing: str | None = None
    index = 0
    while index < len(message):
        char = message[index]
        if opening is None:
            if char == "\x60" and not _escaped(message, index):
                end = index + 1
                while end < len(message) and message[end] == "\x60":
                    end += 1
                opening, closing, index = index, message[index:end], end
                continue
            if char in _QUOTES and not _escaped(message, index):
                opening, closing, index = index, _QUOTES[char], index + 1
                continue
            if char == "'" and not _apostrophe(message, index):
                opening, closing, index = index, char, index + 1
                continue
            index += 1
            continue
        if (
            closing is not None
            and message.startswith(closing, index)
            and not _escaped(message, index)
        ):
            end = index + len(closing)
            if not _active_relay(message, opening, end):
                masked[opening:end] = [" "] * (end - opening)
            opening, closing, index = None, None, end
        else:
            index += 1
    if opening is None:
        return "".join(masked), False
    if not _active_relay(message, opening, None):
        masked[opening:] = [" "] * (len(message) - opening)
    return "".join(masked), True


def _clause_prefix(message: str, position: int, limit: int = 80) -> str:
    prefix = message[max(0, position - limit) : position]
    return prefix[max((prefix.rfind(d) for d in _CLAUSE), default=-1) + 1 :]


def _is_negated(message: str, match: re.Match[str]) -> bool:
    prefix = _clause_prefix(message, match.start())
    context = prefix + match.group(0)
    near, any_word, ownership, suffix = _NEGATION_RULES
    return bool(
        (near.search(prefix) and not _POSITIVE_NEGATION.search(prefix))
        or any_word.search(match.group(0))
        or ownership.search(context)
        or suffix.search(message[match.end() : match.end() + 32])
    )


def durable_owner_requested(message: str) -> bool:
    """Return whether a bounded message requests durable implementation ownership."""
    folded = message.casefold()
    if not any(keyword in folded for keyword in _HINTS):
        return False
    masked, unmatched_quote = _mask_quoted_data(message)
    if unmatched_quote:
        return True
    if not any(keyword in masked.casefold() for keyword in _HINTS):
        return False
    return any(
        any(not _is_negated(masked, match) for match in matcher.finditer(masked))
        for matcher in _DURABLE_RULES
    )
