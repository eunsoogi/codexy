from __future__ import annotations

import csv
import json
import shutil
import subprocess
from time import perf_counter
from pathlib import Path
from unittest.mock import Mock, patch

from codexy_runtime_tools.version_lock import default_package_version


FAKE_MCP = r"""#!/usr/bin/env python3
import json, os, subprocess, sys

if os.environ.get("CODEXY_TEST_PROBE_MODE", "") == "exit-127":
    raise SystemExit(127)
for line in sys.stdin:
    request = json.loads(line)
    if (identifier := request.get("id")) is None:
        continue
    if request["method"] == "initialize":
        params = request.get("params", {})
        if not isinstance(params, dict) or params.get("protocolVersion") != "2024-11-05" or not {"capabilities", "clientInfo"} <= params.keys():
            raise SystemExit(1)
        value = {"serverInfo": {"name": "codexy-" + sys.argv[1], "version": "1.5.1"}}
    elif request["method"] == "tools/list":
        value = {"tools": [{"name": "codegraph_search" if sys.argv[1] == "codegraph" else "lsp_status"}]}
    elif request["method"] == "tools/call" and os.environ.get("CODEXY_TEST_PROBE_MODE", "") == "list-only":
        continue
    elif request["method"] == "tools/call":
        value = {"content": [{"type": "text", "text": "ok"}]}
    else:
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": identifier, "result": value}), flush=True)
"""

DISTRIBUTION_HOST = """#!/usr/bin/env python3
import json, os, subprocess, sys, threading
from pathlib import Path

os.environ.setdefault("CODEXY_MATRIX_STATE", str(Path(os.environ["CODEX_HOME"]).parent / "host-state.json")); os.environ.setdefault("CODEXY_MATRIX_MARKETPLACE", str(Path(os.environ["CODEX_HOME"]).parent / "marketplace")); os.environ.setdefault("CODEXY_MATRIX_VERSION", "fixture")
state_path, root, version = Path(os.environ["CODEXY_MATRIX_STATE"]), Path(os.environ["CODEXY_MATRIX_MARKETPLACE"]).resolve(), os.environ["CODEXY_MATRIX_VERSION"]; state = json.loads(state_path.read_text())
plugins = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}

events = {"PreToolUse": ("preToolUse", "pre_tool_use"), "PermissionRequest": ("permissionRequest", "permission_request"), "UserPromptSubmit": ("userPromptSubmit", "user_prompt_submit")}

def hook_rows():
    rows = []
    for component, plugin_name in plugins.items():
        if component not in state["selection"]: continue
        plugin = root / "plugins" / plugin_name
        path = plugin / "hooks" / "hooks.json"
        if not path.is_file(): continue
        value = json.loads(path.read_text())
        for event, groups in value["hooks"].items():
            for group_index, group in enumerate(groups):
                for hook_index, hook in enumerate(group["hooks"]):
                    command_key = "commandWindows" if os.name == "nt" else "command"
                    command = hook[command_key].replace("${PLUGIN_ROOT}", str(plugin))
                    rows.append({
                        "key": f"{plugin_name}@codexy:hooks/hooks.json:{events[event][1]}:{group_index}:{hook_index}",
                        "eventName": events[event][0],
                        "handlerType": "command",
                        "command": command,
                        "async": hook.get("async", False),
                        "matcher": group.get("matcher"),
                        "timeoutSec": hook.get("timeout", 600),
                        "sourcePath": str(path),
                        "pluginId": f"{plugin_name}@codexy",
                        "enabled": True,
                        "isManaged": False,
                        "currentHash": "sha256:fixture",
                        "trustStatus": "trusted",
                    })
    return rows

if sys.argv[1:][:3] == ["app-server", "--listen", "stdio://"]:
    for line in sys.stdin:
        request = json.loads(line); identifier = request.get("id")
        if request.get("method") == "initialize" and identifier is not None:
            result = {"userAgent": "fixture", "codexHome": "fixture"}
        elif request.get("method") == "hooks/list" and identifier is not None:
            cwds = request.get("params", {}).get("cwds", [])
            result = {"data": [{"cwd": cwds[0] if cwds else ".", "hooks": hook_rows(), "warnings": [], "errors": []}]}
        else:
            continue
        print(json.dumps({"jsonrpc": "2.0", "id": identifier, "result": result}), flush=True)
        if request.get("method") == "hooks/list" and os.name == "nt" and str(Path(os.environ["CODEX_HOME"])).endswith("tree-probe-home"):
            child = subprocess.Popen([sys.executable, "-c", "import threading; threading.Event().wait(30)"], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, close_fds=True); state_path.with_suffix(".pids").write_text(str(child.pid), encoding="utf-8")
    if os.name == "nt" and str(Path(os.environ["CODEX_HOME"])).endswith("tree-probe-home"): threading.Event().wait(30)
    raise SystemExit(0)

def installed(component):
    plugin = plugins[component]
    return {"pluginId": plugin + "@codexy", "name": plugin, "marketplaceName": "codexy", "version": version, "installed": True, "enabled": True, "source": {"source": "local", "path": str(root / "plugins" / plugin)}, "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}

if sys.argv[1:][:4] == ["plugin", "marketplace", "list", "--json"]:
    payload = {"marketplaces": [] if not state["marketplace"] else [{"name": "codexy", "root": str(root), "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}]}
elif sys.argv[1:][:3] == ["plugin", "marketplace", "upgrade"]:
    payload = {"ok": True}
elif sys.argv[1:][:3] in (["plugin", "marketplace", "add"], ["plugin", "marketplace", "remove"]):
    state["marketplace"] = sys.argv[1:][2] == "add"; state_path.write_text(json.dumps(state)); payload = {"ok": True}
elif sys.argv[1:][:3] == ["plugin", "list", "--json"]:
    payload = {"installed": [installed(component) for component in ("core", "github", "devtools") if component in state["selection"]]}
elif sys.argv[1:][:2] in (["plugin", "add"], ["plugin", "remove"]):
    plugin = sys.argv[1:][2].split("@", 1)[0]
    component = next(component for component, name in plugins.items() if name == plugin)
    if sys.argv[1:][1] == "add":
        if state.get("fail_add") == plugin:
            state.pop("fail_add"); state_path.write_text(json.dumps(state)); print(json.dumps({"error": "injected"})); raise SystemExit(1)
        if component not in state["selection"]: state["selection"].append(component)
    elif component in state["selection"]: state["selection"].remove(component)
    state_path.write_text(json.dumps(state)); payload = {"ok": True}
else:
    payload = {"ok": True}
print(json.dumps(payload))
"""


def copy_marketplace_plugins(repository: Path, root: Path) -> str:
    version = default_package_version()
    for plugin in ("codexy", "codexy-github", "codexy-devtools"):
        destination = root / "plugins" / plugin
        shutil.copytree(repository / "plugins" / plugin, destination)
        manifest_path = destination / ".codex-plugin/plugin.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["version"] = version
        manifest_path.write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )
    _git(root, "init", "-q")
    _git(root, "branch", "-M", "main")
    _git(root, "config", "user.name", "fixture")
    _git(root, "config", "user.email", "fixture@example.invalid")
    _git(root, "add", "plugins")
    _git(root, "commit", "-qm", "fixture release")
    tag = f"v{version}"
    _git(root, "tag", tag)
    tag_revision = _git(root, "rev-parse", f"{tag}^{{commit}}")
    (root / "main-marker").write_text("main", encoding="utf-8")
    _git(root, "add", "main-marker")
    _git(root, "commit", "-qm", "fixture main drift")
    main_revision = _git(root, "rev-parse", "main")
    if main_revision == tag_revision:
        raise RuntimeError("fixture main revision unexpectedly equals the release tag")
    _git(root, "checkout", "-q", "--detach", tag)
    (root / ".codex-marketplace-install.json").write_text(
        json.dumps(
            {
                "ref_name": tag,
                "revision": tag_revision,
                "source": "https://github.com/eunsoogi/codexy.git",
                "source_type": "git",
                "sparse_paths": [],
            }
        ),
        encoding="utf-8",
    )
    return version


def measure_hook_probes(marketplace: Path, version: str) -> list[dict[str, object]]:
    from codexy_runtime_tools import component_hook_activation_host as host
    from codexy_runtime_tools.component_capability_probe import probe_component

    host.list_hooks(
        marketplace.parent / "codex.cmd", marketplace.parent / "tree-probe-home"
    )
    assert not host_process_active(marketplace.parent / "host-state.pids")
    assert_cleanup_failures()
    measurements = []
    for component, plugin_name in (("core", "codexy"), ("github", "codexy-github")):
        plugin = marketplace / "plugins" / plugin_name
        started = perf_counter()
        result = probe_component(
            component, plugin, {"name": plugin_name, "version": version}
        )
        measurements.append(
            {
                "component": component,
                "elapsed_seconds": round(perf_counter() - started, 6),
                "category": result.get("_category") or result.get("reason_code"),
                "started": result.get("started"),
                "callable": result.get("callable"),
            }
        )
    return measurements


def windows_argv(probe, root: Path, platform_os):
    launchers = tuple(
        root / directory / "probe.cmd"
        for directory in ("plain", "codexy&staging", "codexy staging")
    )
    for launcher in launchers:
        launcher.parent.mkdir()
        launcher.write_text("@exit /b 0\r\n", encoding="utf-8")
    python = root / "Python Runtime" / "python.exe"
    with patch.object(probe, "os", platform_os):
        batch = tuple(
            probe._argv(f'"{launcher}" PermissionRequest', root)
            for launcher in launchers
        )
        native = (
            probe._argv("powershell.exe -NoProfile -File hook.ps1", root),
            probe._argv(f'"{python}" hook.py', root),
        )
    executed = (
        tuple(subprocess.run(argv, check=False, timeout=5).returncode for argv in batch)
        if probe.os.name == "nt"
        else ()
    )
    return launchers, batch, native, python, executed


def host_process_active(path: Path) -> bool:
    pid = path.read_text(encoding="utf-8").strip()
    result = subprocess.run(
        ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        check=True,
        timeout=5,
    )
    return any(
        len(row) == 5 and row[0].lower().endswith((".exe", ".com")) and row[1] == pid
        for row in csv.reader(result.stdout.splitlines())
    )


def assert_cleanup_failures() -> None:
    import codexy_runtime_tools.component_hook_activation_host as host

    for failure in (
        subprocess.CompletedProcess([], 1),
        subprocess.TimeoutExpired([], 1),
        OSError("taskkill unavailable"),
    ):
        process = Mock(pid=123, poll=Mock(return_value=None))
        with patch.object(host.subprocess, "run", side_effect=[failure]):
            assert not host._terminate_process_tree(process)
        process.kill.assert_called_once_with()


def _git(root: Path, *arguments: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), *arguments], text=True, stderr=subprocess.PIPE
    ).strip()
