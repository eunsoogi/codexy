from __future__ import annotations

import io
import json
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch

from codexy_runtime_tools.component_cli import main


class ComponentCliTests(unittest.TestCase):
    def test_json_install_prints_exactly_one_operation_receipt(self) -> None:
        receipt = {
            "schema": "getcodexy.operation-receipt.v1",
            "operation_id": "op-test",
            "command": "install",
            "outcome": "completed",
            "requested_components": [],
            "resolved_components": ["core", "github", "devtools"],
            "selection_before": [],
            "selection_after": ["core", "github", "devtools"],
            "installed_components": ["core", "github", "devtools"],
            "source_of_truth": "installed-component-inventory",
            "errors": [],
        }
        output = io.StringIO()
        with patch("codexy_runtime_tools.component_cli.run_operation", return_value=receipt) as operation, redirect_stdout(output):
            code = main(["--codex", "/trusted/codex", "install", "--json"])

        self.assertEqual(code, 0)
        self.assertEqual(json.loads(output.getvalue()), receipt)
        self.assertEqual(output.getvalue().count("\n"), 1)
        self.assertEqual(operation.call_args.args[0:2], ("install", ()))

    def test_rejected_operation_has_nonzero_exit_status(self) -> None:
        receipt = {"outcome": "rejected", "errors": [{"code": "missing-removal-target"}]}
        with patch("codexy_runtime_tools.component_cli.run_operation", return_value=receipt), redirect_stdout(io.StringIO()):
            self.assertEqual(main(["--codex", "/trusted/codex", "remove", "--json"]), 2)

    def test_public_install_does_not_require_a_codex_override(self) -> None:
        receipt = {"outcome": "completed", "command": "install"}
        with patch("codexy_runtime_tools.component_cli.run_operation", return_value=receipt) as operation, redirect_stdout(io.StringIO()):
            self.assertEqual(main(["install", "--json"]), 0)
        self.assertIsNone(operation.call_args.args[3])


if __name__ == "__main__":
    unittest.main()
