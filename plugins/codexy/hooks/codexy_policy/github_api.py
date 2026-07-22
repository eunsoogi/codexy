"""Classify mutating GitHub API calls against the owned repository."""

from __future__ import annotations

import re

from .repository import OWNED, github_identity

FIELD_OPTIONS = {"-f", "-F", "--field", "--raw-field"}
VALUE_OPTIONS = {"--cache", "--hostname", "--input", "--preview"}
HEADER_OPTIONS = {"-H", "--header"}
FLAG_OPTIONS = {"--include", "-i", "--paginate", "--slurp", "--silent", "--verbose"}
MUTATION = re.compile(r"(?:^|[\s,{])mutation(?:[\s({]|$)", re.IGNORECASE)
REPOSITORY = re.compile(r"^/?repos/([^/]+)/([^/]+)(?:/|$)", re.IGNORECASE)


def forbidden(args: list[str], default_owned: bool) -> bool:
    parsed = _parse(args)
    if parsed is None:
        return default_owned
    endpoint, method, fields = parsed
    if endpoint.casefold().strip("/") == "graphql":
        query = fields.get("query")
        if query is not None and MUTATION.search(query) is None:
            return False
        return _graphql_owned(query, fields, default_owned)
    if method in {"GET", "HEAD"}:
        return False
    match = REPOSITORY.match(endpoint)
    if match is None:
        return False
    if tuple(part.casefold() for part in match.groups()) == ("{owner}", "{repo}"):
        return default_owned
    return github_identity(f"{match.group(1)}/{match.group(2)}") == OWNED


def _parse(args: list[str]) -> tuple[str, str, dict[str, str]] | None:
    method, fields, positionals, index = None, {}, [], 0
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
            if index + 1 >= len(args) or not _field(fields, args[index + 1]):
                return None
            index += 2
        elif any(token.startswith(option + "=") for option in FIELD_OPTIONS):
            if not _field(fields, token.split("=", 1)[1]):
                return None
            index += 1
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
    return positionals[0], method or ("POST" if fields else "GET"), fields


def _field(fields: dict[str, str], value: str) -> bool:
    name, separator, content = value.partition("=")
    if not separator or not name or name in fields:
        return False
    fields[name] = content
    return True


def _graphql_owned(query: str | None, fields: dict[str, str], default_owned: bool) -> bool:
    owner, name = fields.get("owner"), fields.get("name")
    if owner is not None and name is not None:
        return github_identity(f"{owner}/{name}") == OWNED
    if query is not None and re.search(r"eunsoogi\s*[/,]\s*codexy", query, re.IGNORECASE):
        return True
    return default_owned
