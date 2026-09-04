"""Small JavaScript lexer and literal parser for nested connector admission."""

from __future__ import annotations

from typing import NamedTuple

MAX_TOKENS = 10_000
MAX_TEMPLATE_DEPTH = 32
ESCAPES = {
    "b": "\b",
    "f": "\f",
    "n": "\n",
    "r": "\r",
    "t": "\t",
    "v": "\v",
    "0": "\0",
    "\\": "\\",
    "/": "/",
    "'": "'",
    '"': '"',
}
REGEX_AFTER = frozenset(
    "await case delete do else in new of return throw typeof void yield".split()
)
LINE_TERMINATORS = frozenset("\r\n\u2028\u2029")


class Token(NamedTuple):
    kind: str
    value: str


class ParseError(ValueError):
    pass


def tokenize(source: str) -> list[Token]:
    return _tokenize(source, 0)


def _tokenize(source: str, template_depth: int) -> list[Token]:
    if template_depth > MAX_TEMPLATE_DEPTH:
        raise ParseError("template nesting")
    result: list[Token] = []
    index = 0
    while index < len(source):
        character = source[index]
        if character.isspace():
            index += 1
        elif source.startswith("//", index):
            index = _line_comment_end(source, index + 2)
            if index == len(source):
                break
        elif source.startswith("/*", index):
            end = source.find("*/", index + 2)
            if end == -1:
                raise ParseError("unterminated comment")
            index = end + 2
        elif character in "'\"":
            value, index = _string(source, index)
            result.append(Token("string", value))
        elif character == "`":
            template, index = _template(source, index, template_depth)
            result.extend(template)
        elif character == "/" and _regex_start(result):
            index = _regex(source, index)
            result.append(Token("regex", "regex"))
        elif character == "\\":
            raise ParseError("unicode identifier escape")
        elif character.isalpha() or character in "_$":
            end = index + 1
            while end < len(source) and (source[end].isalnum() or source[end] in "_$"):
                end += 1
            result.append(Token("identifier", source[index:end]))
            index = end
        elif character.isdigit() or (
            character == "-" and index + 1 < len(source) and source[index + 1].isdigit()
        ):
            end = index + 1
            while end < len(source) and source[end].isdigit():
                end += 1
            result.append(Token("number", source[index:end]))
            index = end
        elif source.startswith("...", index):
            result.append(Token("other", "..."))
            index += 3
        else:
            result.append(Token("punctuation", character))
            index += 1
        if len(result) > MAX_TOKENS:
            raise ParseError("too many tokens")
    return result


def _line_comment_end(source: str, index: int) -> int:
    while index < len(source) and source[index] not in LINE_TERMINATORS:
        index += 1
    return index


def _template(source: str, index: int, depth: int) -> tuple[list[Token], int]:
    result = [Token("dynamic", "template")]
    index += 1
    while index < len(source):
        if source[index] == "\\":
            index += 2
        elif source[index] == "`":
            return result, index + 1
        elif source.startswith("${", index):
            start = index + 2
            end = _template_expression_end(source, start, depth)
            result.append(Token("dynamic", "template_expression_start"))
            result.extend(_tokenize(source[start:end], depth + 1))
            result.append(Token("dynamic", "template_expression_end"))
            index = end + 1
        else:
            index += 1
    raise ParseError("unterminated template")


def _template_expression_end(source: str, index: int, depth: int) -> int:
    braces = 1
    while index < len(source):
        character = source[index]
        if character in "'\"":
            _, index = _string(source, index)
        elif character == "`":
            _, index = _template(source, index, depth + 1)
        elif source.startswith("//", index):
            index = _line_comment_end(source, index + 2)
            if index == len(source):
                return len(source)
        elif source.startswith("/*", index):
            end = source.find("*/", index + 2)
            if end == -1:
                raise ParseError("unterminated comment")
            index = end + 2
        elif character == "/" and _source_regex_start(source, index):
            index = _regex(source, index)
        elif character == "{":
            braces += 1
            index += 1
        elif character == "}":
            braces -= 1
            if braces == 0:
                return index
            index += 1
        else:
            index += 1
    raise ParseError("unterminated template expression")


def _source_regex_start(source: str, index: int) -> bool:
    cursor = index - 1
    while cursor >= 0 and source[cursor].isspace():
        cursor -= 1
    if cursor < 0:
        return True
    character = source[cursor]
    if character in ")]}'\"":
        return False
    if character.isalnum() or character in "_$":
        end = cursor + 1
        while cursor >= 0 and (source[cursor].isalnum() or source[cursor] in "_$"):
            cursor -= 1
        return source[cursor + 1 : end] in REGEX_AFTER
    return True


def _regex_start(tokens: list[Token]) -> bool:
    if not tokens:
        return True
    previous = tokens[-1]
    if previous.kind in {"identifier", "number", "string", "regex"}:
        return previous.kind == "identifier" and previous.value in REGEX_AFTER
    return previous.value not in {
        ")",
        "]",
        "}",
    }


def _regex(source: str, index: int) -> int:
    index += 1
    in_class = False
    while index < len(source):
        character = source[index]
        if character in "\r\n":
            raise ParseError("newline in regex")
        if character == "\\":
            index += 2
        elif character == "[":
            in_class = True
            index += 1
        elif character == "]":
            in_class = False
            index += 1
        elif character == "/" and not in_class:
            index += 1
            while index < len(source) and source[index].isalpha():
                index += 1
            return index
        else:
            index += 1
    raise ParseError("unterminated regex")


def _string(source: str, index: int) -> tuple[str, int]:
    quote = source[index]
    index += 1
    result: list[str] = []
    while index < len(source):
        character = source[index]
        if character == quote:
            return "".join(result), index + 1
        if character in "\r\n":
            raise ParseError("newline in string")
        if character != "\\":
            result.append(character)
            index += 1
            continue
        index += 1
        if index >= len(source):
            raise ParseError("unterminated escape")
        escaped = source[index]
        if escaped == "u":
            digits = source[index + 1 : index + 5]
            if len(digits) != 4 or any(
                d not in "0123456789abcdefABCDEF" for d in digits
            ):
                raise ParseError("invalid unicode escape")
            result.append(chr(int(digits, 16)))
            index += 5
        elif escaped in ESCAPES:
            result.append(ESCAPES[escaped])
            index += 1
        else:
            raise ParseError("unsupported escape")
    raise ParseError("unterminated string")
