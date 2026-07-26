"""Classify mutating GitHub API calls against the owned repository."""

from __future__ import annotations

import json
import re

from .repository import OWNED, github_identity, read_text

TYPED_FIELD_OPTIONS = {"-F", "--field"}
FIELD_OPTIONS = {"-f", "--raw-field"} | TYPED_FIELD_OPTIONS
VALUE_OPTIONS = {"--cache", "--hostname", "--preview"}
HEADER_OPTIONS = {"-H", "--header"}
FLAG_OPTIONS = {"--include", "-i", "--paginate", "--slurp", "--silent", "--verbose"}
MUTATION = re.compile(r"(?<![_0-9A-Za-z])mutation(?![_0-9A-Za-z])", re.IGNORECASE)
REPOSITORY = re.compile(r"^/?repos/([^/]+)/([^/]+)(?:/|$)", re.IGNORECASE)


class _UnsafeQueryFile(Exception):
    pass


def forbidden(args: list[str], default_owned: bool, cwd: str) -> bool:
    try:
        parsed = _parse(args, cwd)
    except _UnsafeQueryFile:
        return True
    if parsed is None:
        return default_owned
    endpoint, method, fields, input_file = parsed
    if endpoint.casefold().strip("/") == "graphql":
        if input_file is not None:
            query = _input_query(cwd, input_file)
            return query is None or _mutation(query) is not False
        query = fields.get("query")
        return query is None or _mutation(query) is not False
    if method in {"GET", "HEAD"}:
        return False
    match = REPOSITORY.match(endpoint)
    if match is None:
        return False
    if tuple(part.casefold() for part in match.groups()) == ("{owner}", "{repo}"):
        return default_owned
    return github_identity(f"{match.group(1)}/{match.group(2)}") == OWNED


def _parse(args: list[str], cwd: str) -> tuple[str, str, dict[str, str], str | None] | None:
    method, fields, input_file, positionals, index = None, {}, None, [], 0
    while index < len(args):
        token = args[index]
        if token in {"-X", "--method"}:
            if method is not None or index + 1 >= len(args):
                return None
            method, index = args[index + 1].upper(), index + 2
        elif token.startswith("--method=") or token.startswith("-X="):
            if method is not None:
                return None
            method, index = token.split("=", 1)[1].upper(), index + 1
        elif token.startswith("-X") and len(token) > 2:
            if method is not None:
                return None
            method, index = token[2:].upper(), index + 1
        elif token in FIELD_OPTIONS:
            if index + 1 >= len(args) or not _field(fields, args[index + 1], cwd if token in TYPED_FIELD_OPTIONS else None):
                return None
            index += 2
        elif any(token.startswith(option + "=") for option in FIELD_OPTIONS):
            typed = any(token.startswith(option + "=") for option in TYPED_FIELD_OPTIONS)
            if not _field(fields, token.split("=", 1)[1], cwd if typed else None):
                return None
            index += 1
        elif token == "--input":
            if input_file is not None or index + 1 >= len(args):
                return None
            input_file, index = args[index + 1], index + 2
        elif token.startswith("--input="):
            if input_file is not None:
                return None
            input_file, index = token.split("=", 1)[1], index + 1
        elif token in VALUE_OPTIONS:
            if index + 1 >= len(args):
                return None
            index += 2
        elif any(token.startswith(option + "=") for option in VALUE_OPTIONS):
            index += 1
        elif token in HEADER_OPTIONS:
            if index + 1 >= len(args):
                return None
            index += 2
        elif token.startswith("-H") and len(token) > 2:
            index += 1
        elif token.startswith("--header="):
            index += 1
        elif token in FLAG_OPTIONS:
            index += 1
        elif token.startswith("-"):
            return None
        else:
            positionals.append(token)
            index += 1
    if len(positionals) != 1 or not positionals[0]:
        return None
    return positionals[0], method or ("POST" if fields or input_file is not None else "GET"), fields, input_file


def _field(fields: dict[str, str], value: str, typed_cwd: str | None) -> bool:
    name, separator, content = value.partition("=")
    if not separator or not name or name in fields:
        return False
    if typed_cwd is not None and name == "query" and content.startswith("@"):
        loaded = read_text(typed_cwd, content[1:])
        if loaded is None:
            raise _UnsafeQueryFile
        content = loaded
    fields[name] = content
    return True


def _input_query(cwd: str, target: str) -> str | None:
    content = read_text(cwd, target)
    if content is None:
        return None
    try:
        body = json.loads(content)
    except json.JSONDecodeError:
        return None
    query = body.get("query") if isinstance(body, dict) else None
    return query if isinstance(query, str) else None


def _mutation(query: str) -> bool | None:
    """Classify operation text while ignoring GraphQL comments and string values."""
    visible, index = [], 0
    while index < len(query):
        if query[index] == "#":
            index = query.find("\n", index)
            if index < 0:
                break
            visible.append("\n")
            index += 1
        elif query.startswith('"""', index):
            index += 3
            while index < len(query):
                if query.startswith(r'\"""', index):
                    index += 4
                elif query.startswith('"""', index):
                    index += 3
                    break
                else:
                    index += 1
            else:
                return None
            visible.append(" ")
        elif query[index] == '"':
            index += 1
            while index < len(query):
                if query[index] == "\\":
                    index += 2
                elif query[index] == '"':
                    index += 1
                    break
                elif query[index] in "\r\n":
                    return None
                else:
                    index += 1
            else:
                return None
            visible.append(" ")
        else:
            visible.append(query[index])
            index += 1
    return MUTATION.search("".join(visible)) is not None
