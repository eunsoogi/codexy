"""Lifecycle preflight and storage-boundary cases."""

import errno
import json
import os
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from packages.getcodexy.tests.component_lifecycle_records import record, recorded
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentLifecyclePreflightCases:
    def test_interrupted_mutation_restores_selection_and_record(self) -> None:
        with fixture({"core"}, interrupt_add="codexy-github") as state:
            record(state.home, ["core"])
            receipt = run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-interrupted",
            )

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core"})
            self.assertEqual(recorded(state.home), ["core"])

    def test_stale_record_is_rejected_without_a_host_mutation(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core", "github"])
            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-stale",
            )

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(
                receipt["errors"], [{"code": "inconsistent-installed-state"}]
            )
            self.assertEqual(state.mutations, [])
            saved = inventory_path(state.home).parent / "receipts" / "op-stale.json"
            self.assertEqual(
                json.loads(saved.read_text(encoding="utf-8"))["selection_before"],
                ["core"],
            )

    @unittest.skipIf(
        os.name == "nt", "creating a symlink requires Windows developer privileges"
    )
    def test_symlinked_transaction_storage_is_rejected_without_a_host_mutation(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.symlink_to(state.root / "outside")
            receipt = run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-link",
            )

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(state.mutations, [])

    def test_unsafe_operation_id_is_rejected_before_transaction_storage(self) -> None:
        with fixture() as state:
            with self.assertRaisesRegex(ValueError, "safe op-"):
                run_operation(
                    "install",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="../escape",
                )
            self.assertEqual(state.mutations, [])

    def test_an_existing_lifecycle_lock_refuses_a_second_operation(self) -> None:
        with fixture() as state:
            from codexy_runtime_tools.component_transaction_state import (
                transaction_lock,
            )

            target = "msvcrt.locking" if os.name == "nt" else "fcntl.flock"
            failure = (
                OSError(errno.EACCES, "already locked")
                if os.name == "nt"
                else BlockingIOError()
            )
            with (
                patch(target, side_effect=failure),
                self.assertRaisesRegex(RuntimeError, "another getcodexy"),
            ):
                with transaction_lock(state.home):
                    pass
            self.assertEqual(state.mutations, [])
