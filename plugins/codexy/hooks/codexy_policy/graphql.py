"""Fail-closed GraphQL operation classification for GitHub API admission."""

from __future__ import annotations

def mutation(query: str) -> bool | None:
    """Return whether a syntactically complete document defines a mutation."""
    tokens = _tokens(query)
    if tokens is None:
        return None
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token == "{":
            index = _selection(tokens, index)
        elif token in {"query", "mutation", "subscription", "fragment"}:
            definition = _definition(tokens, index)
            if definition is None:
                return None
            index = definition
            if token == "mutation":
                return True
        else:
            return None
        if index is None:
            return None
    return False


def _tokens(query: str) -> list[str] | None:
    tokens, index = [], 0
    while index < len(query):
        char = query[index]
        if char.isspace() or char == ",":
            index += 1
        elif char == "#":
            newline = query.find("\n", index)
            index = len(query) if newline < 0 else newline + 1
        elif query.startswith('"""', index):
            end = _block_string(query, index + 3)
            if end is None:
                return None
            tokens.append("string")
            index = end
        elif char == '"':
            end = _string(query, index + 1)
            if end is None:
                return None
            tokens.append("string")
            index = end
        elif char == "_" or char.isalpha():
            end = index + 1
            while end < len(query) and (query[end] == "_" or query[end].isalnum()):
                end += 1
            tokens.append(query[index:end])
            index = end
        elif query.startswith("...", index):
            tokens.append("...")
            index += 3
        elif char in "!$&():=@[]{|}":
            tokens.append(char)
            index += 1
        elif char.isdigit() or char == "-":
            end = index + 1
            while end < len(query) and (query[end].isalnum() or query[end] in ".+-"):
                end += 1
            tokens.append("number")
            index = end
        else:
            return None
    return tokens


def _string(query: str, index: int) -> int | None:
    while index < len(query):
        char = query[index]
        if char == '"':
            return index + 1
        if char in "\r\n":
            return None
        if char == "\\":
            if index + 1 >= len(query):
                return None
            escape = query[index + 1]
            if escape in '"\\/bfnrt':
                index += 2
            elif escape == "u" and index + 5 < len(query) and all(
                digit in "0123456789abcdefABCDEF" for digit in query[index + 2 : index + 6]
            ):
                index += 6
            else:
                return None
        else:
            index += 1
    return None


def _block_string(query: str, index: int) -> int | None:
    while index < len(query):
        if query.startswith(r'\"""', index):
            index += 4
        elif query.startswith('"""', index):
            return index + 3
        else:
            index += 1
    return None


def _definition(tokens: list[str], index: int) -> int | None:
    index += 1
    stack: list[str] = []
    while index < len(tokens):
        token = tokens[index]
        if token == "{" and not stack:
            return _selection(tokens, index)
        if not _nest(stack, token) or (token == "..." and not stack):
            return None
        index += 1
    return None


def _selection(tokens: list[str], index: int) -> int | None:
    stack: list[str] = []
    content = False
    while index < len(tokens):
        token = tokens[index]
        if len(stack) == 1 and token == "}" and not content:
            return None
        if len(stack) == 1 and token not in "{}()[]:$&!=@|":
            content = True
        if not _nest(stack, token):
            return None
        if not stack:
            return index + 1
        index += 1
    return None


def _nest(stack: list[str], token: str) -> bool:
    pairs = {"}": "{", ")": "(", "]": "["}
    if token in {"{", "(", "["}:
        stack.append(token)
    elif token in pairs:
        if not stack or stack[-1] != pairs[token]:
            return False
        stack.pop()
    return True
