from __future__ import annotations

import subprocess
import unittest
from copy import deepcopy

from codexy_runtime_tools.component_inspection import doctor, status
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_observed_inventory import (
    observe_installed_inventory,
)

from component_lifecycle_support import fixture, installed
from test_component_inspection import materialize


class MarketplaceFallbackTests(unittest.TestCase):
    def test_fallback_preserves_a_canonical_actual_selection(self) -> None:
        with fixture({"core"}) as state:
            materialize(state, "core")
            observed = status(
                state.home, codex=state.codex, runner=self._marketplace_failure(state)
            )
            result = doctor(
                state.home, codex=state.codex, runner=self._marketplace_failure(state)
            )
        self.assertEqual(observed["installed_components"], ["core"])
        self.assertEqual(observed["errors"], [{"code": "invalid-installed-inventory"}])
        self.assertEqual(
            result["host_readiness"],
            {"state": "error", "missing_requirements": ["codex-marketplace-list"]},
        )

    def test_fallback_rejects_noncanonical_plugin_records(self) -> None:
        manifest = load_component_manifest()
        with fixture({"core"}) as state:
            record = installed(state.marketplace, "core")
            cases = {
                "disabled": {"enabled": False},
                "not-installed": {"installed": False},
                "foreign": {
                    "marketplaceSource": {
                        "sourceType": "git",
                        "source": "https://example.invalid/foreign.git",
                    }
                },
            }
            for name, changes in cases.items():
                with self.subTest(name=name):
                    candidate = deepcopy(record)
                    candidate.update(changes)
                    self.assertEqual(
                        observe_installed_inventory(
                            manifest, {"installed": [candidate]}
                        ).error,
                        "conflicting-installed-state",
                    )

    @staticmethod
    def _marketplace_failure(state: fixture):
        def run(command: list[str]) -> subprocess.CompletedProcess[str]:
            if tuple(command[1:]) == ("plugin", "marketplace", "list", "--json"):
                return subprocess.CompletedProcess(command, 1, "", "unavailable")
            return state.run(command)

        return run


if __name__ == "__main__":
    unittest.main()
