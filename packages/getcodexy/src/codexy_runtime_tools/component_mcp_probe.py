"""Probes for the devtools LSP and Codegraph MCP servers."""

import json

from .component_capability_observation import record_probe
from .component_capability_probe_process import _probe_diagnostics, _rpc
from .component_capability_probe import (
    MCP_SPECS,
    _INITIALIZE_PARAMS,
    _argv,
    _base,
    _failure,
    _outcome,
    _request,
)


def _probe_devtools(_component, plugin, base):
    try:
        config = json.loads((plugin / ".mcp.json").read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError):
        return _failure(base, "capability-not-exposed")
    probes = []
    for server in MCP_SPECS:
        server_config = config.get(server) if isinstance(config, dict) else None
        result = probe_server(server, plugin, server_config)
        record_probe(
            base,
            f"mcp:{server}",
            True,
            bool(result.get("started")),
            bool(result.get("callable")),
        )
        if not result.get("started") or not result.get("callable"):
            result["_capability_probes"] = dict(base["_capability_probes"])
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
    run, responses = _rpc(
        _argv(config["command"], plugin, config.get("args", ())), plugin, requests
    )
    base["_capability_probe"] = _probe_diagnostics(run)
    if 1 not in responses:
        failed = run.category in {"missing-launcher", "nonzero-exit", "timeout"}
        reason = "component-start-failed" if failed else "capability-not-exposed"
        return _failure(base, reason, started=not failed)
    initialized = responses[1].get("result", {})
    listed = responses.get(2, {}).get("result", {})
    info = initialized.get("serverInfo", {}) if isinstance(initialized, dict) else {}
    tools = listed.get("tools", []) if isinstance(listed, dict) else []
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
        or any((call.get("error"), call.get("isError"), result.get("isError")))
        or not isinstance(result.get("content"), list)
    ):
        return _failure(base, "capability-call-failed")
    return _outcome(
        base, runtime_name=info.get("name"), runtime_version=info.get("version")
    )
