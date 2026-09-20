"""Small JSON-RPC helpers used by the single-call runner."""

from __future__ import annotations

import json
from copy import deepcopy
from typing import Any

from .support import SUPPORTED_PROTOCOL_VERSIONS
from .types import ExecutionError, ExpectedResult, ResultKind


def request(method: str, identifier: int | None = None, params=None) -> dict[str, Any]:
    value = {"jsonrpc": "2.0", "method": method}
    if identifier is not None:
        value["id"] = identifier
    if params is not None:
        value["params"] = params
    return value


def pointer_get(value: Any, pointer: str) -> Any:
    current = value
    for raw in pointer[1:].split("/"):
        key = raw.replace("~1", "/").replace("~0", "~")
        if isinstance(current, list):
            try:
                current = current[int(key)]
            except (ValueError, IndexError):
                raise KeyError(pointer) from None
        elif isinstance(current, dict) and key in current:
            current = current[key]
        else:
            raise KeyError(pointer)
    return current


def error(kind: ResultKind, message: str, code=None) -> ExecutionError:
    return ExecutionError(kind=kind, message=message, code=code)


def matches(expected: ExpectedResult, kind: ResultKind, response) -> bool:
    if expected.kind is not kind:
        return False
    if not expected.fields:
        return True
    if not isinstance(response, dict):
        return False
    for pointer, wanted in expected.fields.items():
        try:
            if pointer_get(response, pointer) != wanted:
                return False
        except KeyError:
            return False
    return True


def selected(response: dict[str, Any], fields) -> dict[str, Any]:
    values = {}
    for name, pointer in fields.items():
        try:
            values[name] = deepcopy(pointer_get(response, pointer))
        except KeyError:
            continue
    return values


def parse_line(line: bytes, responses: dict[int, dict[str, Any]], malformed) -> None:
    try:
        value = json.loads(line.decode("utf-8"))
    except (UnicodeDecodeError, ValueError, json.JSONDecodeError):
        malformed.set()
        return
    if (
        not isinstance(value, dict)
        or value.get("jsonrpc") != "2.0"
        or not isinstance(value.get("id"), int)
        or isinstance(value.get("id"), bool)
        or value["id"] not in (1, 2, 3)
    ):
        malformed.set()
        return
    identifier = value["id"]
    if identifier in responses:
        malformed.set()
        return
    responses[identifier] = value


def rpc_error(response: dict[str, Any]) -> ExecutionError | None:
    raw_error = response.get("error")
    if not isinstance(raw_error, dict):
        return None
    code = raw_error.get("code")
    if not isinstance(code, (int, str)) or isinstance(code, bool):
        code = None
    return error(ResultKind.JSON_RPC_ERROR, "server returned a JSON-RPC error", code)


def initialize_error(
    response: dict[str, Any], requested_version: str
) -> ExecutionError | None:
    rpc = rpc_error(response)
    if rpc:
        return rpc
    result = response.get("result")
    if not isinstance(result, dict) or not isinstance(result.get("serverInfo"), dict):
        return error(ResultKind.MALFORMED_RESULT, "malformed initialize result")
    selected_version = result.get("protocolVersion")
    if (
        not isinstance(selected_version, str)
        or selected_version not in SUPPORTED_PROTOCOL_VERSIONS
        or selected_version != requested_version
    ):
        return error(
            ResultKind.UNSUPPORTED_PROTOCOL,
            "server selected an unsupported protocol version",
        )
    return None


def tools_list_error(response: dict[str, Any], tool: str) -> ExecutionError | None:
    rpc = rpc_error(response)
    if rpc:
        return rpc
    listed = response.get("result")
    tools = listed.get("tools") if isinstance(listed, dict) else None
    if not isinstance(tools, list) or any(
        not isinstance(item, dict) or not isinstance(item.get("name"), str)
        for item in tools
    ):
        return error(ResultKind.MALFORMED_RESULT, "malformed tools/list result")
    if tool not in {item["name"] for item in tools}:
        return error(ResultKind.TOOL_ERROR, "requested tool was not advertised")
    return None
