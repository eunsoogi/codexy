from __future__ import annotations

import subprocess
import unittest
from copy import deepcopy

from codexy_runtime_tools.component_inspection import doctor, status
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_observed_inventory import observe_installed_inventory

from component_lifecycle_support import fixture, installed
from test_component_inspection import materialize


class MarketplaceFallbackAdmissionTests(unittest.TestCase):
    def test_resolver_owned_observation_admits_only_one_canonical_compatible_selection(self) -> None:
        manifest = load_component_manifest()
        with fixture({"core"}) as state:
            valid = installed(state.marketplace, "core")
            cases = {
                "enabled": (self._changed(valid, enabled=False), "conflicting-installed-state"),
                "installed": (self._changed(valid, installed=False), "conflicting-installed-state"),
                "identity": (self._changed(valid, pluginId="codexy-github@codexy"), "conflicting-installed-state"),
                "source": (self._changed(valid, marketplaceSource={"sourceType": "git", "source": "https://example.invalid/foreign.git"}), "conflicting-installed-state"),
                "duplicate": ([valid, deepcopy(valid)], "conflicting-installed-state"),
                "unknown": ([self._changed(valid, name="unknown", pluginId="unknown@codexy")], "unknown-installed-component"),
                "dependency": ([installed(state.marketplace, "github")], "inconsistent-installed-state"),
                "future": (self._changed(valid, version="9.0.0"), "component-version-mismatch"),
                "mixed": ([valid, installed(state.marketplace, "github", "1.2.0")], "mixed-version-state"),
            }
            admitted = observe_installed_inventory(manifest, {"installed": [valid]})
            self.assertEqual((admitted.selection, admitted.error), (("core",), None))
            self.assertEqual(observe_installed_inventory(manifest, {"installed": [self._changed(valid, version="1.2.0")]}).error, None)
            for name, (records, code) in cases.items():
                with self.subTest(name=name):
                    observed = observe_installed_inventory(manifest, {"installed": records if isinstance(records, list) else [records]})
                    self.assertEqual(observed.error, code)

    def test_marketplace_failure_keeps_typed_probe_error_and_marks_rejected_observations_unhealthy(self) -> None:
        cases = {
            "enabled": ("core", lambda entry: self._changed(entry, enabled=False), "conflicting-installed-state"),
            "installed": ("core", lambda entry: self._changed(entry, installed=False), "conflicting-installed-state"),
            "identity": ("core", lambda entry: self._changed(entry, pluginId="codexy-github@codexy"), "conflicting-installed-state"),
            "foreign": ("core", lambda entry: self._changed(entry, marketplaceSource={"sourceType": "git", "source": "https://example.invalid/foreign.git"}), "conflicting-installed-state"),
            "duplicate": ("core", lambda entry: [entry, deepcopy(entry)], "conflicting-installed-state"),
            "unknown": ("core", lambda entry: [self._changed(entry, name="unknown", pluginId="unknown@codexy")], "unknown-installed-component"),
            "dependency": ("github", lambda entry: [installed_path(entry, "github")], "inconsistent-installed-state"),
        }
        for name, (component, mutate, code) in cases.items():
            with self.subTest(name=name), fixture({component}) as state:
                materialize(state, component)
                seed = installed(state.marketplace, component)
                records = mutate(seed)
                state.inventory_override = {"installed": records if isinstance(records, list) else [records]}
                result = doctor(state.home, codex=state.codex, runner=self._marketplace_failure(state))
                observed = status(state.home, codex=state.codex, runner=self._marketplace_failure(state))

            self.assertEqual(observed["errors"], [{"code": "codex-marketplace-list"}, {"code": code}])
            self.assertEqual(result["host_readiness"], {"state": "error", "missing_requirements": ["codex-marketplace-list"]})
            if name == "unknown":
                self.assertEqual(result["component_health"], [])
            else:
                self.assertEqual(result["component_health"], [{"component": component, "state": "incompatible", "repair": "repair the Codexy registration, then rerun getcodexy doctor"}])

    @staticmethod
    def _changed(entry: dict[str, object], **changes: object) -> dict[str, object]:
        result = deepcopy(entry)
        result.update(changes)
        return result

    @staticmethod
    def _marketplace_failure(state: fixture):
        def run(command: list[str]) -> subprocess.CompletedProcess[str]:
            if tuple(command[1:]) == ("plugin", "marketplace", "list", "--json"):
                return subprocess.CompletedProcess(command, 1, "", "unavailable")
            return state.run(command)

        return run


def installed_path(entry: dict[str, object], component: str) -> dict[str, object]:
    result = deepcopy(entry)
    plugin = {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"}[component]
    result.update({"name": plugin, "pluginId": f"{plugin}@codexy"})
    result["source"] = {"source": "local", "path": str(entry["source"]["path"]).rsplit("/", 1)[0] + "/" + plugin}  # type: ignore[index]
    return result


if __name__ == "__main__":
    unittest.main()
