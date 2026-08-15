from __future__ import annotations

import json
import unittest
from pathlib import Path


CONTRACT_PATH = (
    Path(__file__).parents[1] / "contracts" / "monolith-migration-contract.json"
)


class MonolithMigrationContractTests(unittest.TestCase):
    def test_contract_freezes_safe_admission_recovery_and_receipt_boundaries(
        self,
    ) -> None:
        contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))

        self.assertEqual(contract["schema"], "getcodexy.monolith-migration-contract.v1")
        self.assertEqual(
            contract["command"]["default_selection"], ["core", "github", "devtools"]
        )
        self.assertEqual(
            contract["admission"]["supported_source"],
            "exact-versioned-monolith-baseline",
        )
        self.assertEqual(
            contract["admission"]["target"], "distinct-lockstep-split-release"
        )
        self.assertEqual(
            contract["idempotency"]["same_resolved_selection"],
            "return-completed-without-host-mutation",
        )
        self.assertEqual(
            contract["rollback"]["recovery_order"][0],
            "remove-selected-split-extensions",
        )
        self.assertEqual(
            contract["receipt"]["schema"],
            "getcodexy.monolith-migration-receipt.v1",
        )
        self.assertEqual(
            set(contract["receipt"]["outcomes"]),
            {"completed", "rejected", "rolled-back"},
        )
        self.assertTrue(
            {"ambiguous-monolith", "modified-monolith", "target-release-unavailable"}
            <= set(contract["error_codes"])
        )


if __name__ == "__main__":
    unittest.main()
