from __future__ import annotations

import json
import shutil
import subprocess
from time import perf_counter
from pathlib import Path

from codexy_runtime_tools.version_lock import default_package_version


OFFICIAL = "https://github.com/eunsoogi/codexy.git"
COMPONENTS = {
    "codexy": "core",
    "codexy-github": "github",
    "codexy-devtools": "devtools",
}

FAKE_MCP = r"""#!/usr/bin/env python3
import json
import os
import sys

server = sys.argv[1]
mode = os.environ.get("CODEXY_TEST_PROBE_MODE", "")
if mode == "exit-127":
    raise SystemExit(127)
for line in sys.stdin:
    request = json.loads(line)
    identifier = request.get("id")
    if identifier is None:
        continue
    method = request["method"]
    if method == "initialize":
        params = request.get("params", {})
        if (
            not isinstance(params, dict)
            or params.get("protocolVersion") != "2024-11-05"
            or not {"capabilities", "clientInfo"} <= params.keys()
        ):
            raise SystemExit(1)
        value = {"serverInfo": {"name": "codexy-" + server, "version": "1.5.1"}}
    elif method == "tools/list":
        value = {"tools": [{"name": "codegraph_search" if server == "codegraph" else "lsp_status"}]}
    elif method == "tools/call" and mode == "list-only":
        continue
    elif method == "tools/call":
        value = {"content": [{"type": "text", "text": "ok"}]}
    else:
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": identifier, "result": value}), flush=True)
"""

DISTRIBUTION_HOST = """#!/usr/bin/env python3
import json, os, sys
from pathlib import Path

state_path = Path(os.environ["CODEXY_MATRIX_STATE"])
root = Path(os.environ["CODEXY_MATRIX_MARKETPLACE"]).resolve()
version = os.environ["CODEXY_MATRIX_VERSION"]
state = json.loads(state_path.read_text())
plugins = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}
reverse = {value: key for key, value in plugins.items()}
args = sys.argv[1:]

def save(): state_path.write_text(json.dumps(state))
def installed(component):
    plugin = plugins[component]
    return {"pluginId": plugin + "@codexy", "name": plugin, "marketplaceName": "codexy", "version": version, "installed": True, "enabled": True, "source": {"source": "local", "path": str(root / "plugins" / plugin)}, "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}

if args[:4] == ["plugin", "marketplace", "list", "--json"]:
    payload = {"marketplaces": [] if not state["marketplace"] else [{"name": "codexy", "root": str(root), "marketplaceSource": {"sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"}}]}
elif args[:3] == ["plugin", "marketplace", "add"]:
    state["marketplace"] = True; save(); payload = {"ok": True}
elif args[:3] == ["plugin", "marketplace", "upgrade"]:
    payload = {"ok": True}
elif args[:3] == ["plugin", "marketplace", "remove"]:
    state["marketplace"] = False; save(); payload = {"ok": True}
elif args[:3] == ["plugin", "list", "--json"]:
    payload = {"installed": [installed(component) for component in ("core", "github", "devtools") if component in state["selection"]]}
elif args[:2] == ["plugin", "add"]:
    plugin = args[2].split("@", 1)[0]
    if state.get("fail_add") == plugin:
        state.pop("fail_add"); save(); print(json.dumps({"error": "injected"})); raise SystemExit(1)
    if reverse[plugin] not in state["selection"]: state["selection"].append(reverse[plugin])
    save(); payload = {"ok": True}
elif args[:2] == ["plugin", "remove"]:
    component = reverse[args[2].split("@", 1)[0]]
    if component in state["selection"]: state["selection"].remove(component)
    save(); payload = {"ok": True}
else:
    payload = {"ok": True}
print(json.dumps(payload))
"""


def copy_marketplace_plugins(repository: Path, root: Path) -> str:
    version = default_package_version()
    for plugin in COMPONENTS:
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
                "source": OFFICIAL,
                "source_type": "git",
                "sparse_paths": [],
            }
        ),
        encoding="utf-8",
    )
    return version


def measure_hook_probes(marketplace: Path, version: str) -> list[dict[str, object]]:
    from codexy_runtime_tools.component_capability_probe import probe_component

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
                "category": "success"
                if result.get("callable")
                else result.get("reason_code"),
                "started": result.get("started"),
                "callable": result.get("callable"),
            }
        )
    return measurements


def _git(root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode:
        raise RuntimeError(result.stderr)
    return result.stdout.strip()
