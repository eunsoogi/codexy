"""Process hook inputs and classify runtime results without exposing values."""

import json
import os
import subprocess
import sys


def handle_payload(event, payload, trace, input_event, runtime_lookup):
    is_object = isinstance(payload, dict)
    event_matches = is_object and payload.get("hook_event_name") == event
    if not is_object or not event_matches:
        trace.record(
            "hook_rejected",
            eventKind=event,
            payloadObject=is_object,
            eventNameMatches=event_matches,
        )
        return 0
    if event in ("Interrupt", "UserPromptSubmit"):
        trace.record(
            "hook_received",
            eventKind=event,
            mainSessionIdPresent=_present(payload.get("session_id")),
            turnIdPresent=_present(payload.get("turn_id"))
            if event == "Interrupt"
            else None,
        )
        input_event(
            event,
            payload,
            lambda argument, body: _invoke(argument, body, trace, runtime_lookup),
        )
        return 0
    tool_name = payload.get("tool_name")
    tool_input = payload.get("tool_input")
    tool_name_matched = isinstance(tool_name, str) and (
        tool_name == "watcher_wait" or tool_name.endswith("__watcher_wait")
    )
    trace.record(
        "hook_received",
        eventKind=event,
        toolNamePresent=isinstance(tool_name, str),
        toolNameMatched=tool_name_matched,
        toolInputPresent=isinstance(tool_input, dict),
    )
    if not tool_name_matched or not isinstance(tool_input, dict):
        return 0
    trace.record(
        "hook_received",
        eventKind=event,
        mainSessionIdPresent=_present(payload.get("session_id")),
        turnIdPresent=_present(payload.get("turn_id")),
        toolUseIdPresent=_present(payload.get("tool_use_id")),
        waitSessionIdPresent=_present(tool_input.get("sessionId")),
        parentTokenPresent=_present(tool_input.get("parentToken")),
    )
    result = _invoke("--hook-pretool", payload, trace, runtime_lookup)
    binding = result.get("requestBinding") if isinstance(result, dict) else None
    binding_present = isinstance(binding, str) and bool(binding)
    trace.record("binding_output", eventKind=event, bindingPresent=binding_present)
    if not binding_present:
        return 0
    updated = dict(tool_input)  # Copy before adding the confirmed wait binding.
    updated["requestBinding"] = binding
    output = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "updatedInput": updated,
        }
    }
    sys.stdout.write(json.dumps(output, separators=(",", ":")))
    return 0


def _present(value):
    return isinstance(value, str) and bool(value)


def _invoke(argument, payload, trace, runtime_lookup):
    try:
        runtime = runtime_lookup()
    except (OSError, TypeError, ValueError):
        trace.record(
            "runtime_result",
            runtimeAvailable=False,
            failureClass="runtime_lookup_error",
        )
        raise
    if runtime is None:
        trace.record(
            "runtime_result", runtimeAvailable=False, failureClass="runtime_unavailable"
        )
        return {}
    try:
        result = subprocess.run(
            [str(runtime), argument],
            input=json.dumps(payload, separators=(",", ":")).encode(),
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=2,
            env=os.environ.copy(),
        )
    except subprocess.TimeoutExpired:
        trace.record("runtime_result", runtimeAvailable=True, failureClass="timeout")
        return {}
    except (OSError, subprocess.SubprocessError):
        trace.record(
            "runtime_result", runtimeAvailable=True, failureClass="spawn_error"
        )
        return {}
    if result.returncode != 0:
        trace.record(
            "runtime_result", runtimeAvailable=True, failureClass="nonzero_exit"
        )
        return {}
    try:
        response = json.loads(result.stdout)
    except (TypeError, ValueError, json.JSONDecodeError):
        trace.record(
            "runtime_result", runtimeAvailable=True, failureClass="invalid_response"
        )
        return {}
    binding = response.get("requestBinding") if isinstance(response, dict) else None
    cancelled = response.get("cancelled") if isinstance(response, dict) else None
    trace.record(
        "runtime_result",
        runtimeAvailable=True,
        bindingPresent=isinstance(binding, str) and bool(binding),
        cancellationMatched=cancelled if type(cancelled) is bool else None,
    )
    return response
