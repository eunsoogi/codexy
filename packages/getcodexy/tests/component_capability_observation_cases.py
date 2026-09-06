"""Regression cases for additive doctor capability observations."""

from __future__ import annotations

import importlib
from unittest.mock import patch


class CapabilityObservationCases:
    def test_mcp_client_version_follows_package_authority(self):
        from codexy_runtime_tools import component_capability_probe as probe

        version_lock = importlib.import_module("codexy_runtime_tools.version_lock")
        self.addCleanup(importlib.reload, probe)
        with patch.object(version_lock, "default_package_version") as version:
            version.return_value = "9.9.9"
            importlib.reload(probe)
            self.assertEqual(probe._INITIALIZE_PARAMS["clientInfo"]["version"], "9.9.9")

    def test_doctor_keeps_direct_probe_scope_separate_from_host_verification(self):
        probe = {
            "started": True,
            "callable": True,
            "_capability_probes": {
                "hook:codexy-thread-delivery": {
                    "started": True,
                    "callable": True,
                }
            },
        }
        with patch(
            "codexy_runtime_tools.component_health._probe_component",
            return_value=probe,
        ):
            entry = self._health(("core",))[0]

        capability = entry["observed"]["capabilities"]["hook:codexy-thread-delivery"]
        self.assertEqual(
            capability["states"],
            {
                "configured": "configured",
                "loaded": "loaded",
                "callable": "callable",
                "verified": "unknown",
            },
        )
        self.assertEqual(capability["source"], "getcodexy-direct-probe")
        self.assertEqual(capability["scope"], "plugin-subprocess")
        self.assertIsNone(capability["host_id"])
        self.assertIsNone(capability["session_id"])

    def test_doctor_marks_codegraph_and_lsp_direct_calls_without_host_claim(self):
        with patch(
            "codexy_runtime_tools.component_health._probe_component",
            return_value={
                "started": True,
                "callable": True,
                "_capability_probes": {
                    "mcp:codegraph": {
                        "configured": True,
                        "started": True,
                        "callable": True,
                    },
                    "mcp:lsp": {
                        "configured": True,
                        "started": True,
                        "callable": True,
                    },
                },
            },
        ):
            entry = self._health(("devtools",))[0]

        for name in ("mcp:codegraph", "mcp:lsp"):
            with self.subTest(capability=name):
                observed = entry["observed"]["capabilities"][name]
                self.assertEqual(observed["states"]["callable"], "callable")
                self.assertEqual(observed["states"]["verified"], "unknown")
                self.assertEqual(observed["scope"], "plugin-subprocess")

    def test_actual_doctor_maps_probe_success_and_failure(self):
        self._probe_patch.stop()
        from codexy_runtime_tools import component_capability_probe as probe
        from codexy_runtime_tools.component_inspection import doctor
        from component_lifecycle_support import fixture
        from packages.getcodexy.tests.capability_probe_cases import materialize

        with fixture({"core"}) as state:
            materialize(state, "core")
            result = doctor(state.home, codex=state.codex, runner=state.run)
        health = result["component_health"][0]
        capability = health["observed"]["capabilities"]["hook:codexy-thread-delivery"]
        self.assertEqual(health["state"], "healthy")
        self.assertEqual(
            capability["states"],
            {
                "configured": "configured",
                "loaded": "loaded",
                "callable": "callable",
                "verified": "unknown",
            },
        )

        with fixture({"core"}) as state:
            materialize(state, "core")
            with patch.object(
                probe,
                "_run",
                return_value=probe._RunResult(9, "", "nonzero-exit"),
            ):
                result = doctor(state.home, codex=state.codex, runner=state.run)
        health = result["component_health"][0]
        capability = health["observed"]["capabilities"]["hook:codexy-thread-delivery"]
        self.assertEqual(health["first_failure_stage"], "callable")
        self.assertEqual(capability["states"]["loaded"], "loaded")
        self.assertEqual(capability["states"]["callable"], "unknown")
        self.assertEqual(capability["source"], "getcodexy-direct-probe")

    def test_doctor_does_not_promote_stale_recorded_observation(self):
        self.records["core"]["capability_observation"] = {
            "source": "previous-session",
            "scope": "current-session",
            "host_id": "old-host",
            "session_id": "old-session",
            "states": {
                "configured": "configured",
                "loaded": "loaded",
                "callable": "callable",
                "verified": "verified",
            },
        }
        entry = self._health(("core",))[0]
        for observed in entry["observed"]["capabilities"].values():
            self.assertNotEqual(observed["states"]["verified"], "verified")
            self.assertIsNone(observed["host_id"])
            self.assertIsNone(observed["session_id"])

    def test_unconfigured_component_capabilities_are_explicitly_unknown(self):
        entry = self._health((), ("core",))[0]
        for observed in entry["observed"]["capabilities"].values():
            self.assertEqual(
                observed["states"],
                {
                    "configured": "unknown",
                    "loaded": "unknown",
                    "callable": "unknown",
                    "verified": "unknown",
                },
            )
            self.assertEqual(observed["source"], "unknown")
            self.assertEqual(observed["scope"], "unknown")
