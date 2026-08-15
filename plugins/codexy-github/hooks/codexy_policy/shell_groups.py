"""Typed parser for the supported shell command and grouping grammar."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal, TypeAlias

OPERATORS = {";", "&&", "||", "|", "&"}
OPEN = {"(": ")", "{": "}"}
CLOSE = set(OPEN.values())
PUNCTUATION = set(";&|(){}")
UNSUPPORTED_CONTROL_HEADS = {"case", "elif", "for", "until", "while"}


class GroupSyntaxError(ValueError):
    """The command uses malformed or unsupported shell composition."""


@dataclass(frozen=True)
class Command:
    tokens: tuple[str, ...]


@dataclass(frozen=True)
class Group:
    kind: Literal["subshell", "brace"]
    body: "Sequence"


@dataclass(frozen=True)
class Conditional:
    condition: "Sequence"
    then_branch: "Sequence"
    else_branch: "Sequence | None"


Node: TypeAlias = Command | Conditional | Group


@dataclass(frozen=True)
class Step:
    node: Node
    following: str


@dataclass(frozen=True)
class Sequence:
    steps: tuple[Step, ...]


def parse(tokens: list[str]) -> Sequence:
    if not tokens:
        return Sequence(())
    parser = _Parser(_expand_punctuation(tokens))
    sequence = parser.sequence(None)
    if parser.index != len(parser.tokens):
        raise GroupSyntaxError("unconsumed shell tokens")
    return sequence


def _expand_punctuation(tokens: list[str]) -> list[str]:
    expanded: list[str] = []
    for token in tokens:
        if token and set(token) <= PUNCTUATION:
            index = 0
            while index < len(token):
                pair = token[index : index + 2]
                if pair in {"&&", "||"}:
                    expanded.append(pair)
                    index += 2
                else:
                    expanded.append(token[index])
                    index += 1
        else:
            expanded.append(token)
    return expanded


class _Parser:
    def __init__(self, tokens: list[str]) -> None:
        self.tokens = tokens
        self.index = 0

    def sequence(self, closing: str | None) -> Sequence:
        steps: list[Step] = []
        current: Node | list[str] | None = None
        while self.index < len(self.tokens):
            token = self.tokens[self.index]
            if token in CLOSE:
                if token != closing:
                    raise GroupSyntaxError("unbalanced shell group")
                self.index += 1
                if closing == "}" and current is not None:
                    raise GroupSyntaxError("brace group lacks a command terminator")
                return self._finish(steps, current, grouped=True)
            if token == "if" and current is None:
                self.index += 1
                current = self._conditional()
                continue
            if token in UNSUPPORTED_CONTROL_HEADS and current is None:
                raise GroupSyntaxError("unsupported control head")
            if token in {"then", "else", "fi"}:
                raise GroupSyntaxError("unexpected conditional delimiter")
            if token in OPEN:
                if (
                    isinstance(current, list)
                    and current
                    and all(value == "!" for value in current)
                ):
                    current = None
                elif current is not None:
                    raise GroupSyntaxError(
                        "group cannot be embedded in a simple command"
                    )
                self.index += 1
                body = self.sequence(OPEN[token])
                current = Group("subshell" if token == "(" else "brace", body)
                continue
            if token in OPERATORS:
                if current is None:
                    raise GroupSyntaxError("operator lacks a command")
                steps.append(Step(self._node(current), token))
                current = None
                self.index += 1
                continue
            if isinstance(current, Group):
                raise GroupSyntaxError(
                    "redirection or suffix after group is unsupported"
                )
            if current is None:
                current = []
            current.append(token)
            self.index += 1
        if closing is not None:
            raise GroupSyntaxError("unterminated shell group")
        return self._finish(steps, current, grouped=False)

    @staticmethod
    def _node(value: Node | list[str]) -> Node:
        return (
            value
            if isinstance(value, (Command, Conditional, Group))
            else Command(tuple(value))
        )

    def _conditional(self) -> Conditional:
        condition, delimiter = self._control_chunk({"then"})
        if delimiter != "then":
            raise GroupSyntaxError("conditional lacks then")
        then_branch, delimiter = self._control_chunk({"else", "fi"})
        else_branch = None
        if delimiter == "else":
            else_branch, delimiter = self._control_chunk({"fi"})
        if delimiter != "fi":
            raise GroupSyntaxError("conditional lacks fi")
        return Conditional(
            parse(condition),
            parse(then_branch),
            parse(else_branch) if else_branch else None,
        )

    def _control_chunk(self, delimiters: set[str]) -> tuple[list[str], str]:
        chunk: list[str] = []
        nested = 0
        while self.index < len(self.tokens):
            token = self.tokens[self.index]
            if token == "if":
                nested += 1
            elif token == "fi":
                if nested == 0 and token in delimiters:
                    self.index += 1
                    return chunk, token
                nested -= 1
            elif token in delimiters and nested == 0:
                self.index += 1
                return chunk, token
            chunk.append(token)
            self.index += 1
        raise GroupSyntaxError("unterminated conditional")

    def _finish(
        self, steps: list[Step], current: Node | list[str] | None, grouped: bool
    ) -> Sequence:
        if current is not None:
            steps.append(Step(self._node(current), ""))
        elif not steps or steps[-1].following not in ({";"} if grouped else {";", "&"}):
            raise GroupSyntaxError("empty group or incomplete shell chain")
        return Sequence(tuple(steps))
