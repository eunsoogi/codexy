"""Small JavaScript lexer and literal parser for nested connector admission."""

from __future__ import annotations

from typing import Any, NamedTuple

MAX_TOKENS = 10_000
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
LITERALS = {"true": True, "false": False, "null": None}


class Token(NamedTuple):
    kind: str
    value: str


class ParseError(ValueError):
    pass


def tokenize(source: str) -> list[Token]:
    result: list[Token] = []
    index = 0
    while index < len(source):
        character = source[index]
        if character.isspace():
            index += 1
        elif source.startswith("//", index):
            index = source.find("\n", index + 2)
            if index == -1:
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
            index = _template_end(source, index)
            result.append(Token("dynamic", "template"))
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


def _template_end(source: str, index: int) -> int:
    index += 1
    while index < len(source):
        if source[index] == "\\":
            index += 2
        elif source[index] == "`":
            return index + 1
        else:
            index += 1
    raise ParseError("unterminated template")


class LiteralParser:
    def __init__(self, tokens: list[Token], index: int) -> None:
        self.tokens, self.index = tokens, index

    def parse(self) -> dict[str, Any]:
        value = self.value(0)
        if not isinstance(value, dict):
            raise ParseError("mutation payload must be an object")
        return value

    def value(self, depth: int) -> Any:
        if depth > 32:
            raise ParseError("literal nesting")
        token = self.peek()
        if token.value == "{":
            return self.object(depth + 1)
        if token.value == "[":
            return self.array(depth + 1)
        token = self.take()
        if token.kind == "string":
            return token.value
        if token.kind == "number":
            return int(token.value)
        if token.kind == "identifier" and token.value in LITERALS:
            return LITERALS[token.value]
        raise ParseError("dynamic literal")

    def object(self, depth: int) -> dict[str, Any]:
        self.expect("{")
        result: dict[str, Any] = {}
        if self.accept("}"):
            return result
        while True:
            key = self.take()
            if key.kind not in {"identifier", "string"} or key.value in result:
                raise ParseError("literal key")
            self.expect(":")
            result[key.value] = self.value(depth)
            if self.accept("}"):
                return result
            self.expect(",")
            if self.accept("}"):
                return result

    def array(self, depth: int) -> list[Any]:
        self.expect("[")
        result: list[Any] = []
        if self.accept("]"):
            return result
        while True:
            result.append(self.value(depth))
            if self.accept("]"):
                return result
            self.expect(",")
            if self.accept("]"):
                return result

    def peek(self) -> Token:
        if self.index >= len(self.tokens):
            raise ParseError("unexpected end")
        return self.tokens[self.index]

    def take(self) -> Token:
        token = self.peek()
        self.index += 1
        return token

    def expect(self, value: str) -> None:
        if self.take().value != value:
            raise ParseError(f"expected {value}")

    def accept(self, value: str) -> bool:
        if self.index < len(self.tokens) and self.tokens[self.index].value == value:
            self.index += 1
            return True
        return False
