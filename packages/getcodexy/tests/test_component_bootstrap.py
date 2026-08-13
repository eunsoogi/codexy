from __future__ import annotations

import unittest

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_transition_model import OperationReceipt
from packages.getcodexy.tests.component_lifecycle_support import fixture


class BootstrapTests(unittest.TestCase):
    def test_typed_idempotent_recovery_from_absent_record(self) -> None:
        with fixture({"core"}) as state:
            receipt = run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id="op-bootstrap")
            repeated = run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id="op-bootstrap-repeat")
        self.assertEqual(receipt["command"], "bootstrap")
        self.assertEqual(receipt["selection_before"], ["core"])
        self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
        self.assertEqual(repeated["selection_after"], ["core", "github", "devtools"])

    def test_operands_have_a_typed_rejection(self) -> None:
        with fixture() as state:
            receipt = run_operation("bootstrap", ("core",), state.home, state.codex, state.run, operation_id="op-bootstrap-operand")
        self.assertEqual(receipt["errors"], [{"code": "components-not-accepted"}])
        OperationReceipt.decode(receipt).validate(load_component_manifest())


if __name__ == "__main__":
    unittest.main()
