"""Bounded JavaScript literal extraction for nested connector arguments."""

from __future__ import annotations

from typing import Any

from .repository_github_exec_parser import ParseError, Token

LITERALS = {"true": True, "false": False, "null": None}


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
