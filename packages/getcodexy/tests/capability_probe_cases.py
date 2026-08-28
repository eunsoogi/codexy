"""Capability-probe cases and safe doubles for the doctor tests."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
from unittest.mock import patch

from codexy_runtime_tools.component_health import health
from codexy_runtime_tools.component_manifest import load_component_manifest


REPOSITORY = Path(__file__).resolve().parents[3]
PLUGIN_NAMES = {
    "core": "codexy",
    "github": "codexy-github",
    "devtools": "codexy-devtools",
}
_HEALTH_FIELDS = ("installed", "configured", "started", "callable", "healthy")


def materialize(
    state, *components: str, version: str = load_component_manifest().version
) -> None:
    """Copy selected fixture plugins without running their launchers."""
    for component in components:
        root = state.marketplace / "plugins" / PLUGIN_NAMES[component]
        if root.exists():
            continue
        source = REPOSITORY / "plugins" / PLUGIN_NAMES[component]
        root.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(source, root)
        manifest = root / ".codex-plugin/plugin.json"
        contents = json.loads(manifest.read_text(encoding="utf-8"))
        contents["version"] = version
        manifest.write_text(json.dumps(contents), encoding="utf-8")


class CapabilityProbeCases:
    def setUp(self) -> None:
        super().setUp()
        self.manifest = load_component_manifest()
        self.records = self._records(self.manifest)
        self._probe_patch = patch(
            "codexy_runtime_tools.component_health._probe_component",
            side_effect=self._successful_probe,
            create=True,
        )
        self._probe_patch.start()
        self.addCleanup(self._probe_patch.stop)

    def test_health_reports_each_live_capability_state(self) -> None:
        result = self._health(tuple(PLUGIN_NAMES))
        self.assertEqual(len(result), 3)
        for entry in result:
            with self.subTest(component=entry["component"]):
                self.assertEqual(entry["state"], "healthy")
                self.assertTrue(all(entry[key] for key in _HEALTH_FIELDS))
                self.assertIsNone(entry["first_failure_stage"])
                self.assertIsNone(entry["reason_code"])
                self.assertFalse(entry["restart_required"])
                self.assertEqual(
                    entry["observed"]["plugin"]["name"],
                    PLUGIN_NAMES[entry["component"]],
                )

    def test_health_reports_first_failure_for_start_call_identity_and_authority(
        self,
    ) -> None:
        cases = (
            (
                "start",
                "core",
                {"started": False, "callable": False},
                "started",
                "component-start-failed",
                True,
            ),
            (
                "call",
                "devtools",
                {"started": True, "callable": False},
                "callable",
                "capability-call-failed",
                False,
            ),
            (
                "identity",
                "github",
                {"started": True, "callable": True, "runtime_version": "9.9.9"},
                "identity",
                "runtime-identity-mismatch",
                True,
            ),
        )
        for name, component, override, stage, reason, restart in cases:
            with self.subTest(case=name):
                with patch(
                    "codexy_runtime_tools.component_health._probe_component",
                    side_effect=self._probe_with(component, override),
                    create=True,
                ):
                    entry = self._health((component,))[0]
                self.assertEqual(entry["state"], "incompatible")
                self.assertFalse(entry["healthy"])
                self.assertEqual(entry["first_failure_stage"], stage)
                self.assertEqual(entry["reason_code"], reason)
                self.assertEqual(entry["restart_required"], restart)

    def test_health_rejects_missing_inventory_and_untrusted_authority(self) -> None:
        missing = self._health((), ("core",))[0]
        self.records["core"]["authority"] = {"state": "stale"}
        invalid = self._health(("core",))[0]
        self.assertEqual(missing["reason_code"], "component-not-installed")
        self.assertFalse(missing["healthy"])
        self.assertEqual(invalid["reason_code"], "artifact-authority-invalid")
        self.assertFalse(invalid["healthy"])
        self.records["core"].pop("authority")
        missing_authority = self._health(("core",))[0]
        self.assertEqual(missing_authority["reason_code"], "artifact-authority-invalid")

    def test_live_probe_calls_registered_hooks_and_mcp_tools(self) -> None:
        self._probe_patch.stop()
        from codexy_runtime_tools.component_cli import _probe_component, _probe_server
        from component_lifecycle_support import fixture

        with fixture({"core", "github"}) as state:
            materialize(state, "core", "github")
            for component in ("core", "github"):
                plugin = state.marketplace / "plugins" / PLUGIN_NAMES[component]
                result = _probe_component(component, plugin, self.records[component])
                with self.subTest(component=component):
                    self.assertTrue(result["started"])
                    self.assertTrue(result["callable"])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            server = root / "fake_mcp.py"
            server.write_text(_FAKE_MCP, encoding="utf-8")
            config = {"command": sys.executable, "args": [str(server), "codegraph"]}
            result = _probe_server("codegraph", root, config)
            self.assertEqual(result["runtime_name"], "codexy-codegraph")
            self.assertTrue(result["callable"])
            config["args"][-1] = "lsp"
            self.assertEqual(
                _probe_server("lsp", root, config)["runtime_name"], "codexy-lsp"
            )
            config["args"][-1] = "codegraph"
            with patch.dict(os.environ, {"CODEXY_TEST_PROBE_MODE": "list-only"}):
                self.assertEqual(
                    _probe_server("codegraph", root, config)["reason_code"],
                    "capability-call-failed",
                )
            with patch.dict(os.environ, {"CODEXY_TEST_PROBE_MODE": "exit-127"}):
                failed = _probe_server("codegraph", root, config)
            self.assertFalse(failed["started"])
            self.assertEqual(failed["reason_code"], "component-start-failed")

    def _records(self, manifest):
        return {
            component: {
                "name": plugin,
                "version": manifest.version,
                "source": {"path": str((REPOSITORY / "plugins" / plugin).resolve())},
                "authority": {"state": "valid"},
            }
            for component, plugin in PLUGIN_NAMES.items()
        }

    def _health(self, installed, selected=None):
        return health(self.manifest, installed, selected, self.records, None, False)

    def _successful_probe(self, component, plugin, record):
        return {
            "started": True,
            "callable": True,
            "runtime_name": f"codexy-{component}",
            "runtime_version": record["version"],
        }

    def _probe_with(self, component, override):
        def probe(current, plugin, record):
            result = self._successful_probe(current, plugin, record)
            if current == component:
                result.update(override)
            return result

        return probe


class CapabilityCliCases:
    def test_doctor_capability_failure_returns_nonzero_exit(self) -> None:
        from codexy_runtime_tools.component_cli import main

        receipt = {
            "schema": "getcodexy.doctor.v1",
            "command": "doctor",
            "outcome": "completed",
            "errors": [],
            "component_health": [
                dict(
                    component="core",
                    healthy=False,
                    started=True,
                    callable=False,
                    first_failure_stage="callable",
                    reason_code="capability-call-failed",
                )
            ],
        }
        with patch("codexy_runtime_tools.component_cli.doctor", return_value=receipt):
            self.assertEqual(main(["doctor", "--json"]), 2)


_FAKE_MCP = r"""#!/usr/bin/env python3
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
