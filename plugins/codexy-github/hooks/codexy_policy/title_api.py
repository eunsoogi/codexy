"""Literal GitHub API title inspection and bounded input-file loading."""

from __future__ import annotations

import json
import re
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from .title_graphql import forbidden as graphql_forbidden
from .titles import issue_title, pr_title


_DYNAMIC = re.compile(r"[$`]|__codexy_(?:command|process)_substitution__")
_FIELD_FLAGS = frozenset({"-f", "-F", "--field", "--raw-field"})
_METHOD_FLAGS = frozenset({"--method", "-X"})
_INPUT_FLAG = "--input"
_VALUE_OPTIONS = frozenset(
    {
        "--cache",
        "--header",
        "-H",
        "--hostname",
        "--jq",
        "--preview",
        "--repo",
        "--template",
    }
)
_MAX_INPUT = 1024 * 1024
_MISSING = object()
_UNREADABLE = object()


@dataclass(frozen=True)
class _Arguments:
    endpoint: str | None
    method: str
    method_explicit: bool
    fields: tuple[str | None, ...]
    input_path: str | None
    input_present: bool


def forbidden(args: list[str], cwd: object = None) -> bool:
    parsed = _parse(args)
    if parsed.endpoint is None:
        return False
    body_present = bool(parsed.fields) or parsed.input_present
    method = parsed.method if parsed.method_explicit or not body_present else "POST"
    if parsed.endpoint == "graphql":
        if method == "GET":
            return False
        present, query = _field(parsed.fields, "query")
        if not present and parsed.input_present:
            payload = _read_input(cwd, parsed.input_path)
            query = payload.get("query") if isinstance(payload, dict) else None
            present = query is not None
        return present and graphql_forbidden(query)
    operation = _api_operation(parsed.endpoint, method)
    if operation is None:
        return False
    kind, create = operation
    predicate = issue_title if kind == "issue" else pr_title
    field_present, field_value = _field(parsed.fields, "title")
    if field_present and (
        not isinstance(field_value, str) or not predicate(field_value)
    ):
        return True
    if parsed.input_present:
        payload = _read_input(cwd, parsed.input_path)
        if payload is _UNREADABLE:
            return create
        input_value = (
            payload.get("title", _MISSING) if isinstance(payload, dict) else _MISSING
        )
        if create:
            return (
                input_value is _MISSING
                or not isinstance(input_value, str)
                or not predicate(input_value)
            )
        if input_value is not _MISSING and input_value is not None:
            return not isinstance(input_value, str) or not predicate(input_value)
        return False
    return create and not field_present


def _parse(args: list[str]) -> _Arguments:
    endpoint = None
    method = "GET"
    method_explicit = False
    fields: list[str | None] = []
    input_path = None
    input_present = False
    end_of_options = False
    index = 0
    while index < len(args):
        token = args[index]
        if not end_of_options and token == "--":
            end_of_options = True
            index += 1
            continue
        if not end_of_options and token in _METHOD_FLAGS:
            value = args[index + 1] if index + 1 < len(args) else None
            if value is None:
                return _Arguments(
                    endpoint,
                    method,
                    method_explicit,
                    tuple(fields),
                    input_path,
                    input_present,
                )
            method, method_explicit = value.upper(), True
            index += 2
            continue
        if not end_of_options and token.startswith("--method="):
            method, method_explicit = token.split("=", 1)[1].upper(), True
            index += 1
            continue
        if not end_of_options and token.startswith("-X") and len(token) > 2:
            method, method_explicit = token[2:].removeprefix("=").upper(), True
            index += 1
            continue
        if not end_of_options and token in _FIELD_FLAGS:
            fields.append(args[index + 1] if index + 1 < len(args) else None)
            index += 2
            continue
        if not end_of_options and any(
            token.startswith(flag + "=") for flag in _FIELD_FLAGS
        ):
            fields.append(token.split("=", 1)[1])
            index += 1
            continue
        if not end_of_options and token == _INPUT_FLAG:
            input_present = True
            input_path = args[index + 1] if index + 1 < len(args) else None
            index += 2
            continue
        if not end_of_options and token.startswith(_INPUT_FLAG + "="):
            input_present = True
            input_path = token.split("=", 1)[1]
            index += 1
            continue
        if not end_of_options and token in _VALUE_OPTIONS:
            index += 2
            continue
        if not end_of_options and any(
            token.startswith(option + "=") for option in _VALUE_OPTIONS
        ):
            index += 1
            continue
        if endpoint is None and (end_of_options or not token.startswith("-")):
            endpoint = token
        index += 1
    return _Arguments(
        endpoint, method, method_explicit, tuple(fields), input_path, input_present
    )


def _api_operation(endpoint: str, method: str) -> tuple[str, bool] | None:
    parts = endpoint.strip("/").split("/")
    if len(parts) < 4 or parts[0] != "repos":
        return None
    resource = parts[3]
    if resource not in {"issues", "pulls"}:
        return None
    kind = "issue" if resource == "issues" else "pr"
    if method == "POST" and len(parts) == 4:
        return kind, True
    if method in {"POST", "PATCH", "PUT"} and len(parts) == 5 and parts[4].isdigit():
        return kind, False
    return None


def _field(fields: Sequence[str | None], name: str) -> tuple[bool, str | None]:
    values = [
        value
        for value in fields
        if isinstance(value, str) and value.startswith(name + "=")
    ]
    if not values:
        return False, None
    if len(values) != 1:
        return True, None
    value = values[0].split("=", 1)[1]
    return True, None if _DYNAMIC.search(value) else value


def _read_input(cwd: object, input_path: str | None) -> object:
    if not isinstance(cwd, str) or not isinstance(input_path, str) or not input_path:
        return _UNREADABLE
    if input_path == "-":
        return _UNREADABLE
    try:
        path = Path(input_path)
        if not path.is_absolute():
            path = Path(cwd) / path
        with path.open("rb") as source:
            raw = source.read(_MAX_INPUT + 1)
        if len(raw) > _MAX_INPUT:
            return _UNREADABLE
        return json.loads(raw.decode("utf-8", "strict"), object_pairs_hook=_pairs)
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
        return _UNREADABLE


def _pairs(items: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in items:
        if key in result:
            raise ValueError("duplicate key")
        result[key] = value
    return result
