"""Basic status and bootstrap CLI scenarios."""

from __future__ import annotations

import io
import json
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_cli import main


class ComponentCliBasicCases:
    def test_json_status_prints_one_live_status_object(self) -> None:
        receipt = {
            "schema": "getcodexy.status.v1",
            "command": "status",
            "outcome": "completed",
        }
        output = io.StringIO()
        with (
            patch(
                "codexy_runtime_tools.component_cli.status", return_value=receipt
            ) as command,
            redirect_stdout(output),
        ):
            code = main(["--codex", "/trusted/codex", "status", "--json"])
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(output.getvalue()), receipt)
        self.assertEqual(output.getvalue().count("\n"), 1)
        self.assertEqual(command.call_args.kwargs["codex"], Path("/trusted/codex"))

    def test_bootstrap_delegates_to_the_default_install_transaction(self) -> None:
        receipt = {"outcome": "completed", "command": "install"}
        with (
            patch(
                "codexy_runtime_tools.component_cli.run_operation", return_value=receipt
            ) as operation,
            redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(main(["bootstrap", "--json"]), 0)
        self.assertEqual(operation.call_args.args[0:2], ("bootstrap", ()))

    def test_human_status_names_the_live_state_and_errors(self) -> None:
        receipt = {
            "outcome": "completed",
            "inventory": {"state": "present"},
            "installed_components": ["core"],
            "inventory_consistency": "inconsistent",
            "errors": [{"code": "mixed-version-state"}],
        }
        output = io.StringIO()
        with (
            patch("codexy_runtime_tools.component_cli.status", return_value=receipt),
            redirect_stdout(output),
        ):
            self.assertEqual(main(["status"]), 2)
        self.assertEqual(
            output.getvalue(),
            "getcodexy status: installed=core; inventory=present; consistency=inconsistent; errors=mixed-version-state\n",
        )

    def test_bootstrap_operands_return_a_structured_rejection(self) -> None:
        output = io.StringIO()
        rejection = {
            "schema": "getcodexy.operation-receipt.v1",
            "operation_id": "op-bootstrap-reject",
            "command": "bootstrap",
            "outcome": "rejected",
            "requested_components": ["core"],
            "resolved_components": [],
            "selection_before": [],
            "selection_after": [],
            "installed_components": [],
            "source_of_truth": "installed-component-inventory",
            "errors": [{"code": "components-not-accepted"}],
        }
        with (
            patch(
                "codexy_runtime_tools.component_cli.run_operation",
                return_value=rejection,
            ),
            redirect_stdout(output),
        ):
            self.assertEqual(main(["bootstrap", "core", "--json"]), 2)
        receipt = json.loads(output.getvalue())
        self.assertEqual(receipt["command"], "bootstrap")
        self.assertEqual(receipt["errors"], [{"code": "components-not-accepted"}])
