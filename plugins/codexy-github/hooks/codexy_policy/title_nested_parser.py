"""Small JavaScript lexer and structural helpers for title inspection."""

from __future__ import annotations

from typing import NamedTuple


class Token(NamedTuple):
    kind: str
    value: str


class ParseError(ValueError):
    pass


def value_end(tokens: list[Token], start: int, end: int) -> int:
    depth = {"{": 0, "[": 0, "(": 0}
    pairs = {")": "(", "]": "[", "}": "{"}
    index = start
    while index < end:
        value = tokens[index].value
        if value in depth:
            depth[value] += 1
        elif value in pairs:
            depth[pairs[value]] -= 1
        elif value == "," and not any(depth.values()):
            break
        index += 1
    return index


def arguments(tokens: list[Token], start: int, end: int) -> list[list[Token]]:
    result: list[list[Token]] = []
    begin = start
    depth = {"(": 0, "[": 0, "{": 0}
    pairs = {")": "(", "]": "[", "}": "{"}
    for index in range(start, end):
        value = tokens[index].value
        if value in depth:
            depth[value] += 1
        elif value in pairs:
            depth[pairs[value]] -= 1
        elif value == "," and not any(depth.values()):
            result.append(tokens[begin:index])
            begin = index + 1
    return result + ([tokens[begin:end]] if begin != end else [])


def matching(tokens: list[Token], start: int, opening: str, closing: str) -> int:
    depth = 0
    for index in range(start, len(tokens)):
        if tokens[index].value == opening:
            depth += 1
        elif tokens[index].value == closing:
            depth -= 1
            if depth == 0:
                return index
    raise ParseError("unbalanced expression")


_REGEX_AFTER = frozenset("( [ { , : ; = ! ? & | + - * % ^ ~ < >".split())
_REGEX_WORDS = frozenset(
    "return throw case delete void typeof instanceof in of yield await".split()
)


def tokenize(source: str, *, strict: bool = True) -> list[Token]:
    """Tokenize executable expressions while discarding data-only literals."""
    result: list[Token] = []
    index = 0
    while index < len(source):
        char = source[index]
        if char.isspace():
            index += 1
        elif source.startswith("//", index):
            index = source.find("\n", index + 2)
            if index < 0:
                break
        elif source.startswith("/*", index):
            index = source.find("*/", index + 2)
            if index < 0:
                if strict:
                    raise ParseError("unterminated comment")
                break
            index += 2
        elif char in "'\"":
            value, index = _string(source, index, strict)
            result.append(Token("string", value))
        elif char == "`":
            expression, index = _template(source, index, strict)
            if expression is not None:
                result.append(Token("template", expression))
        elif char == "/" and _regex_start(result, source, index):
            index = _regex(source, index, strict)
        elif char.isalpha() or char in "_$":
            end = index + 1
            while end < len(source) and (source[end].isalnum() or source[end] in "_$"):
                end += 1
            result.append(Token("identifier", source[index:end]))
            index = end
        elif char.isdigit():
            end = index + 1
            while end < len(source) and source[end].isdigit():
                end += 1
            result.append(Token("number", source[index:end]))
            index = end
        else:
            result.append(Token("punctuation", char))
            index += 1
    return result


def _string(source: str, index: int, strict: bool) -> tuple[str, int]:
    quote = source[index]
    index += 1
    chars: list[str] = []
    while index < len(source):
        char = source[index]
        if char == quote:
            return "".join(chars), index + 1
        if char in "\r\n":
            if strict:
                raise ParseError("newline in string")
            return "".join(chars), index
        if char == "\\":
            index += 1
            if index >= len(source):
                if strict:
                    raise ParseError("unterminated escape")
                return "".join(chars), index
            escaped = source[index]
            if escaped == "u" and index + 4 < len(source):
                digits = source[index + 1 : index + 5]
                try:
                    chars.append(chr(int(digits, 16)))
                    index += 5
                    continue
                except ValueError:
                    if strict:
                        raise ParseError("invalid unicode escape") from None
            chars.append(
                {"n": "\n", "r": "\r", "t": "\t", "\\": "\\"}.get(escaped, escaped)
            )
        else:
            chars.append(char)
        index += 1
    if strict:
        raise ParseError("unterminated string")
    return "".join(chars), index


def _template(source: str, index: int, strict: bool) -> tuple[str | None, int]:
    index += 1
    expressions: list[str] = []
    while index < len(source):
        char = source[index]
        if char == "\\":
            index += 2
            continue
        if char == "`":
            return "\n".join(expressions) if expressions else None, index + 1
        if char == "$" and source[index + 1 : index + 2] == "{":
            end = _template_expression_end(source, index + 2)
            if end is None:
                if strict:
                    raise ParseError("unterminated template expression")
                return "\n".join(expressions) if expressions else None, len(source)
            expressions.append(source[index + 2 : end])
            index = end + 1
            continue
        index += 1
    if strict:
        raise ParseError("unterminated template")
    return "\n".join(expressions) if expressions else None, len(source)


def _template_expression_end(source: str, index: int) -> int | None:
    depth, quote, escaped = 1, None, False
    while index < len(source):
        char = source[index]
        if escaped:
            escaped = False
        elif char == "\\" and quote != "'":
            escaped = True
        elif quote is not None:
            if char == quote:
                quote = None
        elif char in "'\"`":
            quote = char
        elif source.startswith("//", index):
            newline = source.find("\n", index + 2)
            index = len(source) if newline < 0 else newline
            continue
        elif source.startswith("/*", index):
            end = source.find("*/", index + 2)
            if end < 0:
                return None
            index = end + 1
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
        index += 1
    return None


def _regex_start(tokens: list[Token], source: str, index: int) -> bool:
    if index + 1 >= len(source) or source[index + 1] in "*/":
        return False
    if not tokens:
        return True
    previous = tokens[-1]
    return previous.value in _REGEX_AFTER or (
        previous.kind == "identifier" and previous.value in _REGEX_WORDS
    )


def _regex(source: str, index: int, strict: bool) -> int:
    index += 1
    character_class = False
    escaped = False
    while index < len(source):
        char = source[index]
        if escaped:
            escaped = False
        elif char == "\\":
            escaped = True
        elif char == "[":
            character_class = True
        elif char == "]":
            character_class = False
        elif char == "/" and not character_class:
            index += 1
            while index < len(source) and source[index].isalpha():
                index += 1
            return index
        elif char in "\r\n":
            break
        index += 1
    if strict:
        raise ParseError("unterminated regular expression")
    return index
