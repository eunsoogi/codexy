from __future__ import annotations

import errno
import io
import json
import os
import sys
import tempfile
from types import SimpleNamespace
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_cli import main
from codexy_runtime_tools.component_manifest import load_component_manifest


class ComponentCliTests(unittest.TestCase):
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

    def test_bootstrap_json_host_failure_emits_one_closed_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output, errors = io.StringIO(), io.StringIO()
            with redirect_stdout(output), redirect_stderr(errors):
                code = main(
                    [
                        "--codex",
                        str(Path(directory) / "missing-codex"),
                        "bootstrap",
                        "--json",
                    ]
                )

        receipt = json.loads(output.getvalue())
        self.assertEqual(code, 2)
        self.assertEqual(output.getvalue().count("\n"), 1)
        self.assertEqual(errors.getvalue(), "")
        self.assertEqual(receipt["schema"], "getcodexy.operation-receipt.v1")
        self.assertEqual(receipt["command"], "bootstrap")
        self.assertEqual(receipt["outcome"], "rejected")
        self.assertEqual(receipt["errors"], [{"code": "inconsistent-installed-state"}])
        self.assertTrue(
            {error["code"] for error in receipt["errors"]}.issubset(
                load_component_manifest().domain_errors
            )
        )

    def test_bootstrap_json_unsafe_symlink_home_emits_one_closed_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            home.mkdir()
            unsafe = Path(directory) / "unsafe-home"
            os.symlink(home, unsafe, target_is_directory=True)
            output, errors = io.StringIO(), io.StringIO()
            with redirect_stdout(output), redirect_stderr(errors):
                code = main(["--codex-home", str(unsafe), "bootstrap", "--json"])

        receipt = json.loads(output.getvalue())
        self.assertEqual(code, 2)
        self.assertEqual(output.getvalue().count("\n"), 1)
        self.assertEqual(errors.getvalue(), "")
        self.assertEqual(receipt["errors"], [{"code": "inconsistent-installed-state"}])

    def test_bootstrap_json_busy_lifecycle_lock_emits_one_closed_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            codex = Path(directory) / "codex"
            codex.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            codex.chmod(0o700)
            output, errors = io.StringIO(), io.StringIO()
            target = "msvcrt.locking" if os.name == "nt" else "fcntl.flock"
            failure = (
                OSError(errno.EACCES, "already locked")
                if os.name == "nt"
                else BlockingIOError()
            )
            with (
                patch(target, side_effect=failure),
                redirect_stdout(output),
                redirect_stderr(errors),
            ):
                code = main(
                    [
                        "--codex",
                        str(codex),
                        "--codex-home",
                        str(Path(directory) / "home"),
                        "bootstrap",
                        "--json",
                    ]
                )

        receipt = json.loads(output.getvalue())
        self.assertEqual(code, 2)
        self.assertEqual(output.getvalue().count("\n"), 1)
        self.assertEqual(errors.getvalue(), "")
        self.assertEqual(receipt["errors"], [{"code": "inconsistent-installed-state"}])

    def test_bootstrap_json_windows_busy_lifecycle_lock_emits_one_closed_receipt(
        self,
    ) -> None:
        class WindowsOS:
            name = "nt"

            def __getattr__(self, attribute: str) -> object:
                return getattr(os, attribute)

        def deny_lock(*_: object) -> None:
            raise OSError(errno.EACCES, "already locked")

        with tempfile.TemporaryDirectory() as directory:
            codex = Path(directory) / "codex"
            codex.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            codex.chmod(0o700)
            output, errors = io.StringIO(), io.StringIO()
            msvcrt = SimpleNamespace(LK_NBLCK=1, LK_UNLCK=2, locking=deny_lock)
            with (
                patch(
                    "codexy_runtime_tools.component_transaction_state.os", WindowsOS()
                ),
                patch.dict(sys.modules, {"msvcrt": msvcrt}),
                redirect_stdout(output),
                redirect_stderr(errors),
            ):
                code = main(
                    [
                        "--codex",
                        str(codex),
                        "--codex-home",
                        str(Path(directory) / "home"),
                        "bootstrap",
                        "--json",
                    ]
                )

        receipt = json.loads(output.getvalue())
        self.assertEqual(code, 2)
        self.assertEqual(output.getvalue().count("\n"), 1)
        self.assertEqual(errors.getvalue(), "")
        self.assertEqual(receipt["errors"], [{"code": "inconsistent-installed-state"}])

    def test_bootstrap_json_does_not_relabel_a_post_mutation_runtime_failure(
        self,
    ) -> None:
        output, errors = io.StringIO(), io.StringIO()
        with (
            patch(
                "codexy_runtime_tools.component_cli.run_operation",
                side_effect=RuntimeError("durable recovery is required"),
            ),
            redirect_stdout(output),
            redirect_stderr(errors),
        ):
            code = main(["bootstrap", "--json"])

        self.assertEqual(code, 1)
        self.assertEqual(output.getvalue(), "")
        self.assertEqual(
            errors.getvalue(), "getcodexy bootstrap: durable recovery is required\n"
        )

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
        with (
            patch(
                "codexy_runtime_tools.component_cli.run_operation", return_value=receipt
            ) as operation,
            redirect_stdout(output),
        ):
            code = main(["--codex", "/trusted/codex", "install", "--json"])

        self.assertEqual(code, 0)
        self.assertEqual(json.loads(output.getvalue()), receipt)
        self.assertEqual(output.getvalue().count("\n"), 1)
        self.assertEqual(operation.call_args.args[0:2], ("install", ()))

    def test_rejected_operation_has_nonzero_exit_status(self) -> None:
        receipt = {
            "outcome": "rejected",
            "errors": [{"code": "missing-removal-target"}],
        }
        with (
            patch(
                "codexy_runtime_tools.component_cli.run_operation", return_value=receipt
            ),
            redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(main(["--codex", "/trusted/codex", "remove", "--json"]), 2)

    def test_public_install_does_not_require_a_codex_override(self) -> None:
        receipt = {"outcome": "completed", "command": "install"}
        with (
            patch(
                "codexy_runtime_tools.component_cli.run_operation", return_value=receipt
            ) as operation,
            redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(main(["install", "--json"]), 0)
        self.assertIsNone(operation.call_args.args[3])


if __name__ == "__main__":
    unittest.main()
