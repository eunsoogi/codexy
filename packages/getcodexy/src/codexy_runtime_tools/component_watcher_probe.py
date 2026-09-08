"""The core Watcher MCP capability probe."""

import json
import os
import shlex
import subprocess
import time

from .component_capability_observation import record_probe
from .component_capability_probe_process import _probe_diagnostics
from .component_interactive_probe import open_rpc
from .version_lock import default_package_version


_INITIALIZE_PARAMS = {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {"name": "getcodexy", "version": default_package_version()},
}


def probe_watcher(plugin, base):
    try:
        config = json.loads((plugin / ".mcp.json").read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError):
        return _failure(base, "capability-not-exposed")
    server_config = config.get("watcher") if isinstance(config, dict) else None
    if not isinstance(server_config, dict):
        return _failure(base, "capability-not-exposed")
    command = server_config.get("command")
    if not isinstance(command, str):
        return _failure(base, "capability-not-exposed")
    argv = _argv(command, plugin, server_config.get("args", ()))
    assignment = _assignment_id()
    requests = [
        _request("initialize", 1, _INITIALIZE_PARAMS),
        _request("tools/list", 2),
        _request(
            "tools/call",
            3,
            {
                "name": "watcher_open",
                "arguments": {
                    "assignmentId": assignment,
                    "parent": {"id": "getcodexy"},
                    "watcher": {"id": "health-probe"},
                    "targets": [{"threadId": "health-probe"}],
                    "ttlSeconds": 30,
                },
            },
        ),
    ]
    try:
        rpc = open_rpc(argv, plugin)
    except OSError:
        record_probe(base, "mcp:watcher", True, False, False)
        return _failure(base, "component-start-failed", started=False)
    responses = {}
    for request in requests:
        response = rpc.request(request)
        if isinstance(response, dict) and isinstance(response.get("id"), int):
            responses[response["id"]] = response
    if 1 not in responses or 2 not in responses or 3 not in responses:
        run = rpc.close()
        base["_capability_probe"] = _probe_diagnostics(run)
        record_probe(base, "mcp:watcher", True, run.category == "success", False)
        return _failure(
            base,
            "component-start-failed"
            if run.category in {"missing-launcher", "nonzero-exit", "timeout"}
            else "capability-not-exposed",
            started=run.category == "success",
        )
    tools = responses[2].get("result", {}).get("tools", [])
    if not any(
        isinstance(tool, dict) and tool.get("name") == "watcher_health"
        for tool in tools
    ):
        rpc.close()
        record_probe(base, "mcp:watcher", True, True, False)
        return _failure(base, "capability-not-exposed")
    try:
        text = responses[3]["result"]["content"][0]["text"]
        opened = json.loads(text)
        session = opened["sessionId"]
        token = opened["parentToken"]
    except (KeyError, IndexError, TypeError, ValueError, json.JSONDecodeError):
        rpc.close()
        record_probe(base, "mcp:watcher", True, True, False)
        return _failure(base, "capability-call-failed")
    health_request = _request(
        "tools/call",
        4,
        {"name": "watcher_health", "arguments": {"sessionId": session, "token": token}},
    )
    cancel_request = _request(
        "tools/call",
        5,
        {
            "name": "watcher_cancel",
            "arguments": {"sessionId": session, "parentToken": token},
        },
    )
    for request in (health_request, cancel_request):
        response = rpc.request(request)
        if isinstance(response, dict) and isinstance(response.get("id"), int):
            responses[response["id"]] = response
    run = rpc.close()
    follow = {key: responses[key] for key in (4, 5) if key in responses}
    callable_result = (
        4 in follow
        and 5 in follow
        and isinstance(follow[4].get("result"), dict)
        and isinstance(follow[5].get("result"), dict)
        and not follow[4].get("error")
        and not follow[5].get("error")
    )
    record_probe(base, "mcp:watcher", True, True, callable_result)
    if not callable_result:
        base["_capability_probe"] = _probe_diagnostics(run)
        return _failure(base, "capability-call-failed")
    return _outcome(
        base,
        _capability_probes={
            "mcp:watcher": {"configured": True, "started": True, "callable": True}
        },
    )


def _request(method, identifier=None, params=None):
    request = {"jsonrpc": "2.0", "method": method, "params": params or {}}
    request.update({"id": identifier} if identifier is not None else {})
    return request


def _assignment_id():
    return f"getcodexy-health-{os.getpid()}-{time.monotonic_ns()}"


def _argv(command, plugin, args=()):
    values = shlex.split(command.replace("${PLUGIN_ROOT}", str(plugin))) + [
        str(value) for value in (args if isinstance(args, list) else ())
    ]
    if values and values[0].endswith("mcp/codexy-mcp-watcher"):
        entrypoint = values[0]
        resolved = (
            entrypoint
            if os.path.isabs(entrypoint)
            else os.path.join(str(plugin), entrypoint)
        )
        resolved = os.path.normpath(resolved)
        suffixes = (".exe", ".cmd", "") if os.name == "nt" else ("", ".sh")
        for suffix in suffixes:
            candidate = f"{resolved}{suffix}"
            if os.path.isfile(candidate):
                values[0] = candidate
                break
    if (
        os.name == "nt"
        and values[0].lower().endswith((".bat", ".cmd"))
        and os.path.isfile(values[0])
    ):
        shell = os.environ.get("COMSPEC", "cmd.exe")
        command = f'"{values[0]}" {subprocess.list2cmdline(values[1:])}'.rstrip()
        return f'{subprocess.list2cmdline([shell])} /d /s /c "{command}"'
    return values


def _failure(base, reason, *, started=True):
    return {**base, "started": started, "reason_code": reason}


def _outcome(base, **fields):
    return {**base, "started": True, "callable": True, **fields}
