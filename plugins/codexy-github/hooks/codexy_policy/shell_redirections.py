"""Token markers and safety checks for shell redirections."""

from __future__ import annotations

from .execution_context import SINGLE_QUOTED_DOLLAR, safe_output_redirection

QUOTED_REDIRECTIONS = {"<": "\ue001", ">": "\ue002"}
REDIRECTION_FD, UNSAFE_REDIRECTION = "\ue003", "\ue004"
OPERATORS = frozenset({";", "&&", "||", "|", "&", "(", ")", "{", "}"})


def mark_redirection_fd(result: list[str]) -> None:
    start = len(result)
    while start and result[start - 1].isdigit():
        start -= 1
    if start < len(result) and (start == 0 or result[start - 1].isspace()):
        result.insert(start, REDIRECTION_FD)


def _is_redirection(token: str) -> bool:
    return any(char in "<>" for char in token) and set(token) <= set("<>&|-")


def strip_redirections(tokens: list[str]) -> list[str] | None:
    result: list[str] = []
    iterator = iter(tokens)
    preceding_segment = False
    for token in iterator:
        if token.startswith(REDIRECTION_FD) and token[1:].isdigit():
            token = next(iterator, None)
            if token is None:
                return None
        if _is_redirection(token):
            target = next(iterator, None)
            if target is None or target in OPERATORS:
                return None
            if not (
                token.startswith("<")
                and ">" not in token
                or token in {">", ">>", ">|", "&>", "&>>"}
                # shlex preserves this marker, so reject it instead of probing another filename.
                and SINGLE_QUOTED_DOLLAR not in target
                # A prior segment could plant a link after the missing-path check.
                and not preceding_segment
                and safe_output_redirection(token, target)
                or token in {">&", ">&-"}
                and (target.isdigit() or target in {"-", "/dev/null"})
            ):
                result.append(UNSAFE_REDIRECTION)
        else:
            result.append(token.replace("\ue001", "<").replace("\ue002", ">"))
            preceding_segment = preceding_segment or token in OPERATORS
    return result
