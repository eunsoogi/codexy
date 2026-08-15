#!/usr/bin/env python3
"""Parse and validate the JSON-RPC response contract for an MCP server."""

from __future__ import annotations

import json
import sys
from pathlib import Path

EXPECTED_IDENTITIES = {
    "lsp": ("codexy-lsp", "lsp_status"),
    "codegraph": ("codexy-codegraph", "codegraph_index"),
}


def check_text(text: str, server: str) -> str | None:
    """Return the original user-facing diagnostic, or ``None`` on success."""
    expected_identity = EXPECTED_IDENTITIES.get(server)
    if expected_identity is None:
        return f"{server}: unknown MCP smoke target"

    responses: dict[int, dict] = {}
    normalized = text.replace("\r\n", "\n").replace("\r", "\n")
    for raw in normalized.split("\n"):
        if not raw.strip():
            continue
        try:
            message = json.loads(raw)
        except json.JSONDecodeError:
            return f"{server}: non-JSON MCP stdout"
        if not isinstance(message, dict):
            return f"{server}: non-object MCP stdout"
        if "id" not in message:
            if (
                message.get("jsonrpc") != "2.0"
                or not isinstance(message.get("method"), str)
                or "result" in message
                or "error" in message
            ):
                return f"{server}: unexpected id-less MCP stdout"
            continue
        identifier = message["id"]
        if type(identifier) is not int or identifier not in (1, 2):
            return f"{server}: unexpected MCP response id"
        if message.get("jsonrpc") != "2.0":
            return f"{server}: invalid JSON-RPC version for response id {identifier}"
        if identifier in responses:
            return f"{server}: duplicate MCP response id {identifier}"
        if "error" in message or "result" not in message:
            return f"{server}: MCP response id {identifier} is not a successful result"
        responses[identifier] = message
    if set(responses) != {1, 2}:
        return f"{server}: MCP response ids were not correlated exactly once"

    expected_server, expected_tool = expected_identity
    initialize_result = responses[1]["result"]
    if (
        not isinstance(initialize_result, dict)
        or initialize_result.get("serverInfo", {}).get("name") != expected_server
    ):
        return f"{server}: unexpected MCP server identity"

    tools_result = responses[2]["result"]
    tools = tools_result.get("tools") if isinstance(tools_result, dict) else None
    if not isinstance(tools, list) or not any(
        isinstance(tool, dict) and tool.get("name") == expected_tool for tool in tools
    ):
        return f"{server}: expected MCP tool is missing"
    return None


def check_file(response_file: str | Path, server: str) -> str | None:
    """Validate one response file while preserving the shell wrapper's wording."""
    with open(response_file, encoding="utf-8") as stream:
        return check_text(stream.read(), server)


def _valid_response() -> str:
    return (
        "\n  \t\n"
        '{"jsonrpc":"2.0","method":"notifications/message","params":{}}\n'
        '{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codexy-lsp"}}}\n'
        '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"lsp_status"}]}}\n'
    )


PARSER_MATRIX = (
    (
        "boolean id",
        '{"jsonrpc":"2.0","id":true,"result":{}}\n{"jsonrpc":"2.0","id":2,"result":{}}\n',
        "lsp: unexpected MCP response id",
    ),
    (
        "wrong version",
        '{"jsonrpc":"1.0","id":1,"result":{}}\n{"jsonrpc":"2.0","id":2,"result":{}}\n',
        "lsp: invalid JSON-RPC version for response id 1",
    ),
    (
        "duplicate id",
        '{"jsonrpc":"2.0","id":1,"result":{}}\n{"jsonrpc":"2.0","id":1,"result":{}}\n{"jsonrpc":"2.0","id":2,"result":{}}\n',
        "lsp: duplicate MCP response id 1",
    ),
    ("valid", _valid_response(), None),
    (
        "non-JSON",
        'runtime banner\n{"jsonrpc":"2.0","id":1,"result":{}}\n{"jsonrpc":"2.0","id":2,"result":{}}\n',
        "lsp: non-JSON MCP stdout",
    ),
    (
        "id-less structured noise",
        '{"level":"info"}\n{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codexy-lsp"}}}\n{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"lsp_status"}]}}\n',
        "lsp: unexpected id-less MCP stdout",
    ),
    (
        "array",
        '[]\n{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codexy-lsp"}}}\n{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"lsp_status"}]}}\n',
        "lsp: non-object MCP stdout",
    ),
    (
        "string",
        '"info"\n{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codexy-lsp"}}}\n{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"lsp_status"}]}}\n',
        "lsp: non-object MCP stdout",
    ),
    (
        "number",
        '1\n{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codexy-lsp"}}}\n{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"lsp_status"}]}}\n',
        "lsp: non-object MCP stdout",
    ),
    (
        "wrong server",
        '{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codexy-codegraph"}}}\n{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"codegraph_index"}]}}\n',
        "lsp: unexpected MCP server identity",
    ),
    (
        "wrong tool",
        '{"jsonrpc":"2.0","id":1,"result":{"serverInfo":{"name":"codexy-lsp"}}}\n{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"codegraph_index"}]}}\n',
        "lsp: expected MCP tool is missing",
    ),
    (
        "unsolicited id",
        '{"jsonrpc":"2.0","id":1,"result":{}}\n{"jsonrpc":"2.0","id":2,"result":{}}\n{"jsonrpc":"2.0","id":99,"result":{}}\n',
        "lsp: unexpected MCP response id",
    ),
)


def run_matrix() -> None:
    """Exercise all exact parser rows in this one Python process."""
    for name, payload, expected in PARSER_MATRIX:
        actual = check_text(payload, "lsp")
        if actual != expected:
            raise AssertionError(f"{name}: expected {expected!r}, got {actual!r}")


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if args == ["--matrix"]:
        run_matrix()
        return 0
    if len(args) != 2:
        print("response file and server name required", file=sys.stderr)
        return 2
    diagnostic = check_file(args[0], args[1])
    if diagnostic is not None:
        print(diagnostic, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
