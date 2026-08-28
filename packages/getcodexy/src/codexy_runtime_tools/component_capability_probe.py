"""Live, read-only probes for the installed component capabilities."""

from __future__ import annotations

import json
import os
import shlex
import subprocess


_RERUN = "rerun getcodexy doctor"
_REGISTRATION_REPAIR = f"repair the Codexy registration, then {_RERUN}"
_INVENTORY_REPAIR = f"repair installed component inventory, then {_RERUN}"
_START_REPAIR = f"repair the installed launcher/runtime, then {_RERUN}"
_CALL_REPAIR = f"use the reported safe component fallback and {_RERUN}"
_EXPOSED_REPAIR = "repair the Codexy registration, then restart Codex"
_IDENTITY_REPAIR = "reinstall the selected release, then restart Codex"
_RUN_OPTIONS = {"capture_output": True, "text": True, "timeout": 5}
FAILURES = {
    "trusted-inventory-unavailable": (_INVENTORY_REPAIR, False),
    "component-not-installed": ("getcodexy bootstrap", True),
    "component-not-configured": (_REGISTRATION_REPAIR, True),
    "component-start-failed": (_START_REPAIR, True),
    "capability-not-exposed": (_EXPOSED_REPAIR, True),
    "capability-call-failed": (_CALL_REPAIR, False),
    "runtime-identity-mismatch": (_IDENTITY_REPAIR, True),
    "artifact-authority-invalid": ("reinstall from a trusted release artifact", True),
}
HOOK_SPECS = {
    "core": ("PermissionRequest", "codexy-thread-delivery"),
    "github": ("UserPromptSubmit", "codexy-github-workflow-context"),
}
MCP_SPECS = {
    "codegraph": ("codegraph_search", {"query": "capability-doctor", "limit": 1}),
    "lsp": ("lsp_status", {"path": "capability-doctor.unknown"}),
}
_INITIALIZE_PARAMS = {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {"name": "getcodexy", "version": "1.5.1"},
}


def _request(method, identifier=None, params=None):
    request = {"jsonrpc": "2.0", "method": method, "params": params or {}}
    if identifier is not None:
        request["id"] = identifier
    return request


def probe_component(component, plugin, record):
    base = _base(record)
    if plugin is None:
        return base
    probe = _probe_hook if component in HOOK_SPECS else _probe_devtools
    return probe(component, plugin, base)


def _probe_hook(component, plugin, base):
    event, marker = HOOK_SPECS[component]
    payload = (
        {"prompt": "review GitHub issue 723"}
        if component == "github"
        else {"tool_name": "codex_app__send_message_to_thread", "tool_input": {}}
    )
    command = _registered_hook(plugin, event, marker)
    if not command:
        return _failure(base, "capability-not-exposed")
    returncode, stdout, timed = _run(
        _argv(command, plugin),
        plugin,
        json.dumps(payload),
        os.environ | {"PLUGIN_ROOT": str(plugin)},
    )
    if timed:
        return _failure(base, "capability-call-failed")
    if returncode in {-1, 127}:
        return _failure(base, "component-start-failed", started=False)
    if returncode:
        return _failure(base, "capability-call-failed")
    try:
        output = json.loads(stdout.strip().splitlines()[-1])["hookSpecificOutput"]
        valid = output["hookEventName"] == event and (
            component == "core" or "$git-workflow" in output["additionalContext"]
        )
    except (IndexError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        valid = False
    return _outcome(
        base,
        callable=valid,
        reason_code=None if valid else "capability-not-exposed",
    )


def _registered_hook(plugin, event, marker):
    try:
        hooks = json.loads((plugin / "hooks/hooks.json").read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError):
        return None
    groups = hooks.get("hooks", {}).get(event, []) if isinstance(hooks, dict) else []
    for group in groups if isinstance(groups, list) else []:
        hooks = group.get("hooks", []) if isinstance(group, dict) else []
        for hook in hooks if isinstance(hooks, list) else []:
            if not isinstance(hook, dict) or marker not in str(hook.get("command", "")):
                continue
            command = hook.get("commandWindows" if os.name == "nt" else "command")
            if isinstance(command, str):
                return command
    return None


def _probe_devtools(_component, plugin, base):
    try:
        config = json.loads((plugin / ".mcp.json").read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError):
        return _failure(base, "capability-not-exposed")
    probes = []
    for server in MCP_SPECS:
        result = probe_server(
            server, plugin, config.get(server) if isinstance(config, dict) else None
        )
        if not result.get("started") or not result.get("callable"):
            return result
        probes.append(result)
    return _outcome(
        base,
        runtime_name=",".join(str(item["runtime_name"]) for item in probes),
        runtime_version=probes[0]["runtime_version"],
        runtime_names=[item["runtime_name"] for item in probes],
        runtime_versions=[item["runtime_version"] for item in probes],
    )


def probe_server(server, plugin, config):
    base = _base()
    if not isinstance(config, dict) or not isinstance(config.get("command"), str):
        return _failure(base, "capability-not-exposed")
    target, extra = MCP_SPECS[server]
    arguments = {"root": str(plugin), **extra}
    requests = (
        _request("initialize", 1, _INITIALIZE_PARAMS),
        _request("notifications/initialized"),
        _request("tools/list", 2),
        _request("tools/call", 3, {"name": target, "arguments": arguments}),
    )
    returncode, responses, timed = _rpc(
        _argv(config["command"], plugin, config.get("args", ())), plugin, requests
    )
    if responses is None or 1 not in responses:
        reason = (
            "component-start-failed"
            if returncode or timed
            else "capability-not-exposed"
        )
        return _failure(base, reason, started=returncode == 0 and not timed)
    info = _response_result(responses[1]).get("serverInfo", {})
    tools = _response_result(responses.get(2, {})).get("tools", [])
    if (
        not isinstance(info, dict)
        or not isinstance(tools, list)
        or not any(
            isinstance(tool, dict) and tool.get("name") == target for tool in tools
        )
    ):
        return _failure(base, "capability-not-exposed")
    call = responses.get(3, {})
    result = call.get("result") if isinstance(call, dict) else None
    if (
        not isinstance(result, dict)
        or call.get("error")
        or call.get("isError")
        or result.get("isError")
        or not isinstance(result.get("content"), list)
    ):
        return _failure(base, "capability-call-failed")
    return _outcome(
        base,
        runtime_name=info.get("name"),
        runtime_version=info.get("version"),
    )


def _base(record=None):
    return dict(
        started=False,
        callable=False,
        runtime_name=record.get("name") if record else None,
        runtime_version=record.get("version") if record else None,
    )


def _failure(base, reason, *, started=True):
    return {**base, "started": started, "reason_code": reason}


def _outcome(base, **fields):
    return {**base, "started": True, "callable": True, **fields}


def _response_result(response):
    return response.get("result") if isinstance(response.get("result"), dict) else {}


def _run(argv, cwd, input_text, env=None):
    try:
        result = subprocess.run(
            argv, input=input_text, cwd=cwd, env=env, **_RUN_OPTIONS
        )
    except subprocess.TimeoutExpired as error:
        return -1, error.stdout or "", True
    except OSError:
        return -1, "", False
    return result.returncode, result.stdout, False


def _rpc(argv, cwd, requests):
    returncode, stdout, timed = _run(
        argv, cwd, "\n".join(json.dumps(request) for request in requests) + "\n"
    )
    values = {}
    for line in (stdout or "").splitlines():
        try:
            value = json.loads(line)
        except (ValueError, json.JSONDecodeError):
            continue
        if isinstance(value, dict) and isinstance(value.get("id"), int):
            values[value["id"]] = value
    return returncode, values, timed


def _argv(command, plugin, args=()):
    return shlex.split(command.replace("${PLUGIN_ROOT}", str(plugin))) + [
        str(value) for value in (args if isinstance(args, list) else ())
    ]


def probe_reason(probe, default):
    reason = probe.get("reason_code")
    return reason if isinstance(reason, str) and reason in FAILURES else default


def identity_matches(manifest, component, record, probe):
    names = probe.get("runtime_names")
    versions = probe.get("runtime_versions")
    return (
        (record or {}).get("name") == manifest.component(component).plugin
        and probe.get("runtime_version") == manifest.version
        and (not names or set(names) == {"codexy-codegraph", "codexy-lsp"})
        and (not versions or set(versions) == {manifest.version})
    )
