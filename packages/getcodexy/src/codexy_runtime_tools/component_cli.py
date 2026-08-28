"""Public command line for transactional component lifecycle operations."""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
from pathlib import Path

from .component_inspection import doctor, status
from .component_lifecycle import PreAdmissionError, run_operation
from .component_transaction_identity import operation_id
from .component_transition_model import OperationReceipt
from .component_transition_rejections import Rejection, RejectionStage, StateFailure
from .monolith_migration import migrate
from .version_lock import default_package_version


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="getcodexy", allow_abbrev=False)
    parser.add_argument(
        "--codex",
        type=Path,
        help="optional absolute path supplied by the trusted Codex host",
    )
    parser.add_argument(
        "--codex-home",
        type=Path,
        default=Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")),
    )
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("install", "update", "remove", "migrate"):
        child = commands.add_parser(command, allow_abbrev=False)
        child.add_argument("components", nargs="*")
        child.add_argument("--json", action="store_true", dest="json_output")
    for command in ("status", "doctor"):
        child = commands.add_parser(command, allow_abbrev=False)
        child.add_argument("--json", action="store_true", dest="json_output")
    bootstrap = commands.add_parser("bootstrap", allow_abbrev=False)
    bootstrap.add_argument("components", nargs="*")
    bootstrap.add_argument("--json", action="store_true", dest="json_output")
    arguments = parser.parse_args(argv)
    try:
        if arguments.command == "status":
            receipt = status(arguments.codex_home, codex=arguments.codex)
        elif arguments.command == "doctor":
            receipt = doctor(arguments.codex_home, codex=arguments.codex)
        elif arguments.command == "migrate":
            if arguments.codex is None:
                raise PreAdmissionError("migrate requires a trusted Codex executable")
            receipt = migrate(
                arguments.codex_home,
                arguments.codex,
                lambda command: _run_migration(command, arguments.codex_home),
                tuple(arguments.components),
            )
        else:
            receipt = run_operation(
                arguments.command,
                tuple(getattr(arguments, "components", ())),
                arguments.codex_home,
                arguments.codex,
            )
    except PreAdmissionError as error:
        if arguments.command == "bootstrap" and arguments.json_output:
            print(json.dumps(_bootstrap_host_failure(), sort_keys=True))
            return 2
        if arguments.command == "migrate" and arguments.json_output:
            print(json.dumps(_migration_host_failure(), sort_keys=True))
            return 2
        print(f"getcodexy {arguments.command}: {error}", file=sys.stderr)
        return 1
    except Exception as error:
        print(f"getcodexy {arguments.command}: {error}", file=sys.stderr)
        return 1
    if arguments.json_output:
        print(json.dumps(receipt, sort_keys=True))
    else:
        print(_human(arguments.command, receipt))
    unhealthy = arguments.command in {"status", "doctor"} and (
        bool(receipt.get("errors"))
        or any(
            isinstance(entry, dict) and entry.get("healthy") is False
            for entry in receipt.get("component_health", [])
        )
    )
    return 0 if receipt["outcome"] == "completed" and not unhealthy else 2


def _run_migration(command: list[str], home: Path):
    from .pre_session import _run

    return _run(command, home)


def _human(command: str, receipt: dict[str, object]) -> str:
    if command == "status":
        return "getcodexy status: installed={installed}; inventory={inventory}; consistency={consistency}; errors={errors}".format(
            installed=",".join(receipt.get("installed_components", [])) or "none",
            inventory=receipt.get("inventory", {}).get("state", "unknown")
            if isinstance(receipt.get("inventory"), dict)
            else "unknown",
            consistency=receipt.get("inventory_consistency", "unknown"),
            errors=",".join(
                error.get("code", "unknown")
                for error in receipt.get("errors", [])
                if isinstance(error, dict)
            )
            or "none",
        )
    if command == "doctor":
        health = receipt.get("component_health", [])
        summary = (
            ",".join(
                f"{entry.get('component')}={entry.get('state')}:{entry.get('repair', 'none')}"
                for entry in health
                if isinstance(entry, dict)
            )
            or "none"
        )
        readiness = receipt.get("host_readiness", {})
        missing = (
            ",".join(readiness.get("missing_requirements", []))
            if isinstance(readiness, dict)
            else "unknown"
        )
        return f"getcodexy doctor: health={summary}; missing={missing or 'none'}; errors={','.join(error.get('code', 'unknown') for error in receipt.get('errors', []) if isinstance(error, dict)) or 'none'}"
    return f"getcodexy {command}: {receipt['outcome']}"


def _probe_component(component: str, plugin: Path | None, record: dict[str, object]) -> dict[str, object]:
    base = {"started": False, "callable": False, "runtime_name": record.get("name"), "runtime_version": record.get("version")}
    if plugin is None: return base
    return _probe_hook(component, plugin, base) if component in {"core", "github"} else _probe_devtools(plugin, base)
def _probe_hook(component: str, plugin: Path, base: dict[str, object]) -> dict[str, object]:
    command = _registered_hook(plugin, component)
    if not command:
        return {**base, "started": True, "reason_code": "capability-not-exposed"}
    payload = {"prompt": "review GitHub issue 723"} if component == "github" else {"tool_name": "codex_app__send_message_to_thread", "tool_input": {}}
    try:
        env = os.environ.copy()
        env["PLUGIN_ROOT"] = str(plugin)
        result = subprocess.run(_argv(command, plugin), input=json.dumps(payload), cwd=plugin, env=env, capture_output=True, text=True, timeout=5, check=False)
    except subprocess.TimeoutExpired:
        return {**base, "started": True, "reason_code": "capability-call-failed"}
    except OSError:
        return {**base, "reason_code": "component-start-failed"}
    if result.returncode != 0:
        return {**base, "started": True, "reason_code": "capability-call-failed"}
    try:
        response = json.loads(result.stdout.strip().splitlines()[-1])
        output = response["hookSpecificOutput"]
        valid = output["hookEventName"] == ("PermissionRequest" if component == "core" else "UserPromptSubmit")
        valid = valid and (component == "core" or "$git-workflow" in output["additionalContext"])
    except (IndexError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        valid = False
    return {**base, "started": True, "callable": valid, "reason_code": None if valid else "capability-not-exposed"}
def _registered_hook(plugin: Path, component: str) -> str | None:
    try:
        hooks = json.loads((plugin / "hooks/hooks.json").read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError):
        return None
    event, marker = (("PermissionRequest", "codexy-thread-delivery") if component == "core" else ("UserPromptSubmit", "codexy-github-workflow-context"))
    groups = hooks.get("hooks", {}).get(event, []) if isinstance(hooks, dict) else []
    for group in groups if isinstance(groups, list) else []:
        for hook in group.get("hooks", []) if isinstance(group, dict) else []:
            if isinstance(hook, dict) and marker in str(hook.get("command", "")):
                return hook.get("commandWindows" if os.name == "nt" else "command")
    return None
def _probe_devtools(plugin: Path, base: dict[str, object]) -> dict[str, object]:
    try:
        config = json.loads((plugin / ".mcp.json").read_text(encoding="utf-8"))
    except (OSError, ValueError, json.JSONDecodeError):
        return {**base, "started": True, "reason_code": "capability-not-exposed"}
    probes = []
    for server in ("codegraph", "lsp"):
        result = _probe_server(server, plugin, config.get(server) if isinstance(config, dict) else None)
        if not result.get("started") or not result.get("callable"):
            return result
        probes.append(result)
    return {**base, "started": True, "callable": True, "runtime_name": ",".join(str(item["runtime_name"]) for item in probes), "runtime_version": probes[0]["runtime_version"], "runtime_names": [item["runtime_name"] for item in probes], "runtime_versions": [item["runtime_version"] for item in probes]}
def _probe_server(server: str, plugin: Path, config: object) -> dict[str, object]:
    base = {"started": False, "callable": False, "runtime_name": None, "runtime_version": None}
    if not isinstance(config, dict) or not isinstance(config.get("command"), str):
        return {**base, "started": True, "reason_code": "capability-not-exposed"}
    target = "codegraph_search" if server == "codegraph" else "lsp_status"
    arguments = {"root": str(plugin), "query": "capability-doctor", "limit": 1} if server == "codegraph" else {"root": str(plugin), "path": "capability-doctor.unknown"}
    requests = ({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}, {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}, {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}, {"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": target, "arguments": arguments}})
    returncode, responses, timed = _rpc(_argv(config["command"], plugin, config.get("args", ())), plugin, requests)
    if responses is None or 1 not in responses:
        return {**base, "reason_code": "component-start-failed" if returncode not in (0, None) or timed else "capability-not-exposed"}
    info = responses[1].get("result", {}).get("serverInfo", {})
    tools = responses.get(2, {}).get("result", {}).get("tools", [])
    if not isinstance(info, dict) or not isinstance(tools, list) or not any(isinstance(tool, dict) and tool.get("name") == target for tool in tools):
        return {**base, "started": True, "reason_code": "capability-not-exposed"}
    call = responses.get(3, {})
    result = call.get("result") if isinstance(call, dict) else None
    if not isinstance(result, dict) or call.get("error") or call.get("isError") or result.get("isError") or not isinstance(result.get("content"), list):
        return {**base, "started": True, "reason_code": "capability-call-failed"}
    return {"started": True, "callable": True, "runtime_name": info.get("name"), "runtime_version": info.get("version")}
def _rpc(argv: list[str], cwd: Path, requests: tuple[dict[str, object], ...]):
    try:
        process = subprocess.Popen(argv, cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    except OSError:
        return None, None, False
    timed = False
    try:
        stdout, _ = process.communicate("\n".join(json.dumps(request) for request in requests) + "\n", timeout=5)
    except subprocess.TimeoutExpired as error:
        timed = True
        process.kill()
        stdout, _ = process.communicate()
        stdout = error.stdout or stdout
    values = {}
    for line in (stdout or "").splitlines():
        try: value = json.loads(line)
        except (ValueError, json.JSONDecodeError): continue
        if isinstance(value, dict) and isinstance(value.get("id"), int): values[value["id"]] = value
    return process.returncode, values, timed
def _argv(command: str, plugin: Path, args: object = ()) -> list[str]:
    values = args if isinstance(args, list) else []
    return shlex.split(command.replace("${PLUGIN_ROOT}", str(plugin))) + [str(value) for value in values]
def _bootstrap_host_failure() -> dict[str, object]:
    rejection = Rejection.from_failure(
        RejectionStage.HOST, StateFailure.INCONSISTENT_INSTALLED_STATE
    )
    return OperationReceipt.rejected(
        operation_id(None), "bootstrap", (), (), rejection
    ).encode()


def _migration_host_failure() -> dict[str, object]:
    return {
        "schema": "getcodexy.monolith-migration-receipt.v1",
        "command": "migrate",
        "outcome": "rejected",
        "source_version": None,
        "target_version": default_package_version(),
        "selection_after": [],
        "errors": [{"code": "trusted-codex-required"}],
        "recovery": "rerun from the trusted Codex host",
    }


if __name__ == "__main__":
    raise SystemExit(main())
