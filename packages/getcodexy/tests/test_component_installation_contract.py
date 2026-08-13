from __future__ import annotations

import json
import unittest
from pathlib import Path


CONTRACT_PATH = (
    Path(__file__).parents[1] / "contracts" / "component-installation-contract.json"
)
FIXTURES_PATH = Path(__file__).parent / "fixtures" / "component-installation-cases.json"


class ComponentInstallationContractTests(unittest.TestCase):
    def test_public_contract_freezes_component_dependencies_and_cli_semantics(self) -> None:
        contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
        manifest = json.loads(
            (Path(__file__).parents[1] / "src/codexy_runtime_tools/component-manifest.json").read_text(encoding="utf-8")
        )

        self.assertEqual(
            contract["schema"], "getcodexy.component-installation-contract.v1"
        )
        self.assertEqual(contract["components"], ["core", "github", "devtools"])
        self.assertEqual(
            contract["component_manifest"],
            {
                "schema": "getcodexy.component-manifest.v1",
                "package_resource": "codexy_runtime_tools/component-manifest.json",
            },
        )
        self.assertEqual(
            contract["dependencies"],
            {"core": [], "github": ["core"], "devtools": ["core"]},
        )
        self.assertEqual(contract["commands"]["install"]["no_arguments"], "all")
        self.assertEqual(
            contract["commands"]["update"]["selection"], "preserve-installed"
        )
        self.assertTrue(contract["commands"]["remove"]["requires_components"])
        self.assertEqual(contract["machine_readable_output"]["flag"], "--json")
        self.assertEqual(
            contract["commands"]["rollback"]["kind"],
            "automatic-mutation-failure-recovery",
        )
        self.assertEqual(
            contract["commands"]["rollback"]["manual_command"],
            "deferred-to-issue-557",
        )
        self.assertIn(
            "operation_id",
            contract["machine_readable_output"]["required_mutation_receipt_fields"],
        )
        self.assertEqual(
            contract["component_products"],
            {"core": "codexy", "github": "codexy-github", "devtools": "codexy-devtools"},
        )
        self.assertEqual(
            contract["machine_readable_output"]["doctor_schema"],
            "getcodexy.doctor.v1",
        )
        self.assertEqual(set(contract["domain_errors"]), set(manifest["domainErrors"]))

    def test_contract_fixtures_cover_happy_risky_regression_and_external_paths(self) -> None:
        fixtures = {
            fixture["id"]: fixture
            for fixture in json.loads(FIXTURES_PATH.read_text(encoding="utf-8"))["fixtures"]
        }

        self.assertEqual(fixtures["install-default"]["selection_after"], ["core", "github", "devtools"])
        self.assertEqual(fixtures["install-github"]["selection_after"], ["core", "github"])
        self.assertEqual(fixtures["update-preserves-selection"]["selection_after"], ["core", "devtools"])
        transitions = {transition["id"]: transition for transition in json.loads(FIXTURES_PATH.read_text(encoding="utf-8"))["state_transitions"]}
        self.assertEqual(
            transitions["update-explicit-preserves"]["selection_after"],
            ["core", "github", "devtools"],
        )
        self.assertEqual(fixtures["remove-core-with-dependent"]["error"]["code"], "dependency-protected-removal")
        self.assertEqual(fixtures["rollback-after-operation-failure"]["outcome"], "rolled-back")
        self.assertEqual(
            fixtures["rollback-after-operation-failure"]["stdout"]["operation_id"],
            "op-rollback-fixture",
        )
        self.assertEqual(fixtures["status-json"]["stdout"]["schema"], "getcodexy.status.v1")
        self.assertEqual(
            fixtures["doctor-json"]["stdout"]["host_readiness"]["state"],
            "ready",
        )
        self.assertEqual(
            fixtures["doctor-json"]["stdout"]["component_health"],
            [{"component": "core", "state": "healthy"}, {"component": "github", "state": "healthy"}],
        )
        self.assertEqual(
            fixtures["status-absent-json"]["stdout"]["inventory_consistency"],
            "not-recorded",
        )
        self.assertEqual(
            fixtures["status-present-empty-json"]["stdout"]["inventory"]["components"],
            [],
        )
        self.assertEqual(
            fixtures["status-inconsistent-json"]["stdout"]["errors"][0]["code"],
            "inconsistent-installed-state",
        )

    def test_every_state_transition_has_deterministic_selection_and_receipt_contract(self) -> None:
        contract = json.loads(FIXTURES_PATH.read_text(encoding="utf-8"))

        for transition in contract["state_transitions"]:
            with self.subTest(transition=transition["id"]):
                self.assertEqual(transition["source_of_truth"], "installed-component-inventory")
                self.assertIn(transition["outcome"], {"completed", "rejected", "rolled-back"})
                self.assertEqual(
                    transition["selection_before"],
                    self._canonical(transition["selection_before"]),
                )
                self.assertEqual(
                    transition["selection_after"],
                    self._canonical(transition["selection_after"]),
                )
                if transition["outcome"] == "rejected":
                    self.assertEqual(transition["selection_before"], transition["selection_after"])
                    self.assertIn("error", transition)

    @staticmethod
    def _canonical(selection: list[str]) -> list[str]:
        return [component for component in ("core", "github", "devtools") if component in selection]


if __name__ == "__main__":
    unittest.main()
