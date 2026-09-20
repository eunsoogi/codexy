"""MCP handshake, single-tool call, and selected-field persistence."""

from __future__ import annotations

import json
import threading
from typing import Any

from .process import run_bounded
from .protocol import (
    error,
    initialize_error,
    matches,
    parse_line,
    request,
    rpc_error,
    selected,
    tools_list_error,
)
from .support import MAX_REQUEST_BYTES
from .types import (
    ExecutionError,
    ExecutionResult,
    ResultKind,
    ScenarioValidationError,
    SingleCall,
)


def _result(
    call: SingleCall,
    kind: ResultKind,
    capture,
    stored=None,
    error=None,
    response=None,
) -> ExecutionResult:
    stored = stored or {}
    return ExecutionResult(
        kind=kind,
        stored=stored,
        meets_expectation=matches(call.expected, kind, response),
        error=error,
        returncode=capture.returncode,
        elapsed_seconds=capture.elapsed_seconds,
        tool=call.tool,
        protocol_version=call.protocol_version,
        transport=call.transport,
        observed_output_bytes=capture.stdout_bytes + capture.stderr_bytes,
    )


def _encoded(value: dict[str, Any]) -> bytes:
    return json.dumps(value, separators=(",", ":")).encode() + b"\n"


def _request_payloads(call: SingleCall) -> tuple[bytes, bytes, bytes, bytes]:
    payloads = (
        _encoded(
            request(
                "initialize",
                1,
                {
                    "protocolVersion": call.protocol_version,
                    "capabilities": {},
                    "clientInfo": {"name": "codexy-mcp-scenario", "version": "1.0"},
                },
            )
        ),
        _encoded(request("notifications/initialized")),
        _encoded(request("tools/list", 2, {})),
        _encoded(
            request(
                "tools/call", 3, {"name": call.tool, "arguments": dict(call.arguments)}
            )
        ),
    )
    if sum(map(len, payloads)) > MAX_REQUEST_BYTES:
        raise ScenarioValidationError("encoded MCP request exceeds the request limit")
    return payloads


def run_single_call(call: SingleCall, cancellation: Any = None) -> ExecutionResult:
    """Run one explicit initialize/list/call exchange against local stdio."""

    if cancellation is not None:
        if hasattr(cancellation, "is_cancelled"):
            cancelled = cancellation.is_cancelled
        elif hasattr(cancellation, "is_set"):
            cancelled = cancellation.is_set
        else:
            raise ScenarioValidationError(
                "cancellation must expose is_cancelled or is_set"
            )
    else:
        cancelled = None

    initialize_bytes, initialized_bytes, list_bytes, call_bytes = _request_payloads(
        call
    )
    responses: dict[int, dict[str, Any]] = {}
    malformed = threading.Event()
    stop_event = threading.Event()
    phase_errors: list[ExecutionError] = []

    def on_line(line: bytes, send_input) -> None:
        previous = set(responses)
        parse_line(line, responses, malformed)
        if malformed.is_set():
            stop_event.set()
            return
        new_ids = set(responses) - previous
        if len(new_ids) != 1:
            malformed.set()
            stop_event.set()
            return
        identifier = new_ids.pop()
        if identifier == 1:
            issue = initialize_error(responses[1], call.protocol_version)
            if issue:
                phase_errors.append(issue)
                stop_event.set()
            else:
                send_input(initialized_bytes)
                send_input(list_bytes)
        elif identifier == 2:
            issue = tools_list_error(responses[2], call.tool)
            if issue:
                phase_errors.append(issue)
                stop_event.set()
            else:
                send_input(call_bytes)
        elif identifier == 3:
            stop_event.set()
        else:
            malformed.set()
            stop_event.set()

    capture = run_bounded(
        call.argv,
        call.cwd,
        call.environment,
        initialize_bytes,
        call.timeout_seconds,
        call.output_limit_bytes,
        cancelled,
        stop_event,
        on_line,
    )
    if capture.reason == "timeout":
        return _result(
            call,
            ResultKind.TIMEOUT,
            capture,
            error=error(ResultKind.TIMEOUT, "call timed out"),
        )
    if capture.reason == "cancelled":
        return _result(
            call,
            ResultKind.CANCELLED,
            capture,
            error=error(ResultKind.CANCELLED, "call cancelled"),
        )
    if capture.reason == "output-limit":
        return _result(
            call,
            ResultKind.OUTPUT_LIMIT,
            capture,
            error=error(
                ResultKind.OUTPUT_LIMIT, "process output exceeded the declared limit"
            ),
        )
    if capture.launch_error:
        return _result(
            call,
            ResultKind.LAUNCH_ERROR,
            capture,
            error=error(ResultKind.LAUNCH_ERROR, "launcher unavailable"),
        )
    if malformed.is_set():
        return _result(
            call,
            ResultKind.MALFORMED_RESULT,
            capture,
            error=error(ResultKind.MALFORMED_RESULT, "invalid JSON-RPC result"),
        )
    if phase_errors:
        issue = phase_errors[0]
        return _result(call, issue.kind, capture, error=issue)
    if not all(identifier in responses for identifier in (1, 2, 3)):
        kind = (
            ResultKind.PROCESS_ERROR
            if capture.returncode not in (None, 0)
            else ResultKind.MALFORMED_RESULT
        )
        return _result(
            call,
            kind,
            capture,
            error=error(kind, "required MCP response was not received"),
        )

    call_response = responses[3]
    call_error = rpc_error(call_response)
    if call_error:
        return _result(
            call,
            ResultKind.JSON_RPC_ERROR,
            capture,
            stored=selected(call_response, call.stored_fields),
            error=call_error,
            response=call_response,
        )
    result = call_response.get("result")
    if not isinstance(result, dict) or not isinstance(result.get("content"), list):
        kind = ResultKind.MALFORMED_RESULT
        return _result(
            call,
            kind,
            capture,
            stored=selected(call_response, call.stored_fields),
            error=error(kind, "malformed tools/call result"),
            response=call_response,
        )
    kind = (
        ResultKind.TOOL_ERROR if result.get("isError") is True else ResultKind.SUCCESS
    )
    execution_error = (
        error(kind, "tool reported an error") if kind is ResultKind.TOOL_ERROR else None
    )
    return _result(
        call,
        kind,
        capture,
        stored=selected(call_response, call.stored_fields),
        error=execution_error,
        response=call_response,
    )
