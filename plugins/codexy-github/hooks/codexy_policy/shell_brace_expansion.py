"""Bounded Bash brace-word expansion for shared command-policy parsing."""

from __future__ import annotations

import re

from .shell_redirections import (
    BRACE_CLOSE,
    BRACE_OPEN,
    BRACE_UNSUPPORTED,
    BRACE_WORD,
)

MAX_PRODUCTS = 128
MAX_NESTING = 8
MAX_STEPS = 32
_NO_EXPANSION = object()
_RANGE = re.compile(r"^(?P<start>-?[0-9]+|[A-Za-z])\.\.(?P<end>-?[0-9]+|[A-Za-z])$")


def mark(command: str) -> str | None:
    """Mark only active braces before shlex removes quote and escape context."""
    markers = {BRACE_OPEN, BRACE_CLOSE, BRACE_UNSUPPORTED, BRACE_WORD}
    if any(marker in command for marker in markers):
        return None
    result: list[str] = []
    quote: str | None = None
    escaped = False
    depth = 0
    for char in command:
        if escaped:
            result.append(char)
            escaped = False
        elif char == "\\" and quote != "'":
            # Quotes or escapes inside a brace word need semantics this bounded parser does not model.
            if depth:
                result.append(BRACE_UNSUPPORTED)
            result.append(char)
            escaped = True
        elif char in {"'", '"'}:
            if depth:
                result.append(BRACE_UNSUPPORTED)
            quote = None if quote == char else char if quote is None else quote
            result.append(char)
        elif quote is None and char == "{":
            result.append(BRACE_OPEN)
            depth += 1
        elif quote is None and char == "}" and depth:
            result.append(BRACE_CLOSE)
            depth -= 1
        else:
            result.append(char)
    return "".join(result)


def expand(tokens: list[str]) -> list[str] | None:
    """Expand bounded words while marking products as data, never new syntax."""
    result: list[str] = []
    total_products = 0
    for token in tokens:
        if token == BRACE_OPEN:
            result.append("{")
            continue
        if token == BRACE_CLOSE:
            result.append("}")
            continue
        expanded = _expand_word(token, 0)
        if expanded is None:
            return None
        words, did_expand = expanded
        if did_expand:
            # Bash applies these expansions after braces; policy parsing must not miss their new words.
            if any(_has_unmodeled_followup_expansion(word) for word in words):
                return None
            total_products += len(words)
            if total_products > MAX_PRODUCTS:
                return None
            result.extend(BRACE_WORD + word for word in words)
        else:
            result.extend(words)
    return result


def is_product(token: str) -> bool:
    """Tell shell evaluation which words were created after syntax parsing."""
    return token.startswith(BRACE_WORD)


def restore(token: str) -> str:
    """Remove private parser markers only after operators and prefixes are handled."""
    return (
        token.removeprefix(BRACE_WORD)
        .replace(BRACE_UNSUPPORTED, "")
        .replace(BRACE_OPEN, "{")
        .replace(BRACE_CLOSE, "}")
    )


def _expand_word(token: str, steps: int) -> tuple[list[str], bool] | None:
    if steps > MAX_STEPS or len(token) > 4096:
        return None
    pair = _first_pair(token)
    if pair is None:
        if BRACE_OPEN in token or BRACE_CLOSE in token:
            return None
        return [restore(token)], False
    start, end, nesting = pair
    if nesting > MAX_NESTING:
        return None
    inner = token[start + 1 : end]
    choices = _choices(inner)
    if choices is None:
        return None
    if choices is _NO_EXPANSION:
        # Bash leaves non-expanding braces literal; nested active pairs still need inspection.
        literal = token[:start] + "{" + inner + "}" + token[end + 1 :]
        return _expand_word(literal, steps + 1)

    products: list[str] = []
    for choice in choices:
        nested = _expand_word(token[:start] + choice + token[end + 1 :], steps + 1)
        if nested is None or len(products) + len(nested[0]) > MAX_PRODUCTS:
            return None
        products.extend(nested[0])
    return products, True


def _first_pair(token: str) -> tuple[int, int, int] | None:
    start: int | None = None
    depth = maximum = 0
    for index, char in enumerate(token):
        if char == BRACE_OPEN:
            if depth == 0:
                start = index
            depth += 1
            maximum = max(maximum, depth)
        elif char == BRACE_CLOSE:
            if depth == 0 or start is None:
                return None
            depth -= 1
            if depth == 0:
                return start, index, maximum
    return None


def _choices(inner: str) -> list[str] | None | object:
    if BRACE_UNSUPPORTED in inner and _top_level_parts(inner) is not None:
        return None
    parts = _top_level_parts(inner)
    if parts is not None:
        return parts
    sequence = _sequence(inner)
    if sequence is not _NO_EXPANSION:
        return sequence
    # A step range is active in some Bash versions but unsupported by this bounded parser.
    if ".." in inner:
        return None
    return _NO_EXPANSION


def _top_level_parts(value: str) -> list[str] | None:
    parts: list[str] = []
    start = depth = 0
    for index, char in enumerate(value):
        if char == BRACE_OPEN:
            depth += 1
        elif char == BRACE_CLOSE:
            depth -= 1
            if depth < 0:
                return None
        elif char == "," and depth == 0:
            parts.append(value[start:index])
            start = index + 1
    if depth:
        return None
    if parts:
        parts.append(value[start:])
        return parts
    return None


def _sequence(value: str) -> list[str] | None | object:
    match = _RANGE.fullmatch(value)
    if match is None:
        return _NO_EXPANSION
    start, end = match.group("start"), match.group("end")
    if start.isalpha() != end.isalpha():
        return _NO_EXPANSION
    if start.isalpha():
        first, last = ord(start), ord(end)
        if (start.islower() != end.islower()) or abs(last - first) + 1 > MAX_PRODUCTS:
            return None
        step = 1 if first <= last else -1
        return [chr(code) for code in range(first, last + step, step)]

    try:
        first, last = int(start), int(end)
    except ValueError:
        return None
    count = abs(last - first) + 1
    if count > MAX_PRODUCTS:
        return None
    step = 1 if first <= last else -1
    width = max(len(start.lstrip("-")), len(end.lstrip("-")))
    padded = any(
        len(value.lstrip("-")) > 1 and value.lstrip("-").startswith("0")
        for value in (start, end)
    )
    return [
        _format_integer(value, width if padded else 0)
        for value in range(first, last + step, step)
    ]


def _format_integer(value: int, width: int) -> str:
    sign = "-" if value < 0 else ""
    digits = str(abs(value))
    return sign + digits.zfill(width) if width else sign + digits


def _has_unmodeled_followup_expansion(word: str) -> bool:
    """Fail closed when brace products can still change under later Bash expansions."""
    if any(character in word for character in "$`*?[]"):
        return True
    return word.startswith("~") or "=~" in word or ":~" in word
