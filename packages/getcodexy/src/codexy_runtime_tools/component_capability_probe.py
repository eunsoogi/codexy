"""Live, read-only probes for the installed component capabilities."""

import json
import os
import shlex
import subprocess

from .component_hook_activation import ACTIVATION_REPAIRS
from .component_capability_observation import record_probe
from .component_capability_probe_process import (
    _RUN_OPTIONS,
    _RunResult,
    _probe_diagnostics,
    _run,
)
from .component_watcher_probe import probe_watcher as _probe_watcher
from .version_lock import default_package_version


_RERUN = "rerun getcodexy doctor"
_REGISTRATION_REPAIR = f"repair the Codexy registration, then {_RERUN}"
_INVENTORY_REPAIR = f"repair installed component inventory, then {_RERUN}"
_START_REPAIR = f"repair the installed launcher/runtime, then {_RERUN}"
_CALL_REPAIR = f"use the reported safe component fallback and {_RERUN}"
_EXPOSED_REPAIR = "repair the Codexy registration, then restart Codex"
_IDENTITY_REPAIR = "reinstall the selected release, then restart Codex"
FAILURES = {
    "trusted-inventory-unavailable": (_INVENTORY_REPAIR, False),
    "component-not-installed": ("getcodexy bootstrap", True),
    "component-not-configured": (_REGISTRATION_REPAIR, True),
    "component-start-failed": (_START_REPAIR, True),
    "capability-not-exposed": (_EXPOSED_REPAIR, True),
    "capability-call-failed": (_CALL_REPAIR, False),
    "runtime-identity-mismatch": (_IDENTITY_REPAIR, True),
    "artifact-authority-invalid": ("reinstall from a trusted release artifact", True),
    **ACTIVATION_REPAIRS,
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
    "clientInfo": {"name": "getcodexy", "version": default_package_version()},
}


def _request(method, identifier=None, params=None):
    request = {"jsonrpc": "2.0", "method": method, "params": params or {}}
    request.update({"id": identifier} if identifier is not None else {})
    return request


def probe_component(component, plugin, record):
    base = _base(record)
    if plugin is None:
        return base
    if component == "core":
        return _probe_core(plugin, base)
    probe = _probe_hook if component in HOOK_SPECS else _probe_devtools
    return probe(component, plugin, base)


def _probe_core(plugin, base):
    hook = _probe_hook("core", plugin, base)
    if not hook.get("started") or not hook.get("callable"):
        return hook
    watcher = _probe_watcher(plugin, base)
    probes = dict(hook.get("_capability_probes", {}))
    probes.update(watcher.get("_capability_probes", {}))
    watcher["_capability_probes"] = probes
    if not watcher.get("started") or not watcher.get("callable"):
        return watcher
    return _outcome(base, _capability_probes=probes)


def _probe_hook(component, plugin, base):
    event, marker = HOOK_SPECS[component]
    capability = f"hook:{marker}"
    payload = {"prompt": "review GitHub issue 723"}
    if component == "core":
        payload = {"tool_name": "codex_app__send_message_to_thread", "tool_input": {}}
    command = _registered_hook(plugin, event, marker)
    if not command:
        record_probe(base, capability, False, False, False)
        return _failure(base, "capability-not-exposed")
    result = _run(
        _argv(command, plugin),
        plugin,
        json.dumps(payload),
        os.environ | {"PLUGIN_ROOT": str(plugin)},
    )
    base["_capability_probe"] = _probe_diagnostics(result)
    base["_category"] = result.category
    if result.category == "missing-launcher":
        record_probe(base, capability, True, False, False)
        return _failure(base, "component-start-failed", started=False)
    if result.category in {"timeout", "nonzero-exit"}:
        record_probe(base, capability, True, True, False)
        return _failure(base, "capability-call-failed")
    try:
        output = json.loads(result.stdout.strip().splitlines()[-1])[
            "hookSpecificOutput"
        ]
        valid = output["hookEventName"] == event and (
            component == "core" or "$git-workflow" in output["additionalContext"]
        )
    except (IndexError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        valid = False
    if not valid:
        base["_category"] = "malformed-output"
        base["_capability_probe"]["category"] = "malformed-output"
    record_probe(base, capability, True, True, valid)
    reason = None if valid else "capability-not-exposed"
    return _outcome(base, callable=valid, reason_code=reason)


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


def _argv(command, plugin, args=()):
    values = shlex.split(command.replace("${PLUGIN_ROOT}", str(plugin))) + [
        str(value) for value in (args if isinstance(args, list) else ())
    ]
    if (
        os.name == "nt"
        and values[0].lower().endswith((".bat", ".cmd"))
        and os.path.isfile(values[0])
    ):
        shell = os.environ.get("COMSPEC", "cmd.exe")
        command = f'"{values[0]}" {subprocess.list2cmdline(values[1:])}'.rstrip()
        return f'{subprocess.list2cmdline([shell])} /d /s /c "{command}"'
    return values


from .component_mcp_probe import _probe_devtools, probe_server  # noqa: E402


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
