from __future__ import annotations

import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import Mock, patch

from codexy_runtime_tools import component_transaction_receipts as receipt_storage
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_resolver import ComponentResolutionError
from codexy_runtime_tools.component_transaction_state import decode_inventory
from codexy_runtime_tools.component_transaction_identity import operation_id
from codexy_runtime_tools.component_transition_model import InventorySnapshot, Journal, OperationReceipt, plan_transition
from codexy_runtime_tools.component_transition_rejections import Rejection, RejectionStage, variants as rejection_variants


class TransitionModelTests(unittest.TestCase):
    def test_every_reachable_terminal_transition_round_trips(self) -> None:
        manifest = load_component_manifest()
        for before in manifest.compatible_combinations:
            for command, requested in _requests():
                for recorded in (None, before):
                    with self.subTest(before=before, command=command, requested=requested, recorded=recorded):
                        try:
                            plan = plan_transition(manifest, command, requested, before, recorded)
                        except ComponentResolutionError:
                            continue
                        snapshot = InventorySnapshot(None if recorded is None else _inventory(before))
                        journal = Journal.decode(plan.journal(_identifier(command, before, requested, recorded), snapshot).encode())
                        journal.validate(manifest, decode_inventory)
                        for outcome, after in (("completed", plan.target), ("rolled-back", plan.before)):
                            receipt = OperationReceipt.decode(journal.receipt(outcome, after).encode())
                            receipt.validate(manifest)

    def test_every_closed_rejection_variant_has_a_valid_receipt(self) -> None:
        manifest = load_component_manifest()
        for before in manifest.compatible_combinations:
            for command, requested in _requests():
                for variant in rejection_variants(manifest, command, requested, before, plan_transition):
                    with self.subTest(before=before, command=command, requested=requested, variant=variant):
                        OperationReceipt.rejected(_identifier(command, before, requested, None), command, requested, before, variant).validate(manifest)

    def test_decoded_receipts_reject_single_field_mutations_and_unknown_values(self) -> None:
        manifest = load_component_manifest()
        plan = plan_transition(manifest, "install", ("github",), ("core",), ("core",))
        receipt = plan.journal("op-mutation", InventorySnapshot(_inventory(("core",)))).receipt("completed").encode()
        mutations = {
            "schema": "getcodexy.operation-receipt.v0",
            "command": "bootstrap",
            "outcome": "pending",
            "requested_components": ["github", "github"],
            "resolved_components": ["core"],
            "selection_before": ["github"],
            "selection_after": ["core"],
            "installed_components": [],
            "source_of_truth": "host",
            "errors": [{"code": "operation-failed"}],
        }
        for field, value in mutations.items():
            with self.subTest(field=field):
                payload = dict(receipt)
                payload[field] = value
                with self.assertRaises(ValueError):
                    OperationReceipt.decode(payload).validate(manifest)
        with self.assertRaises(ValueError):
            OperationReceipt.decode(receipt | {"unknown": True})

    def test_rejected_receipts_accept_only_reachable_stage_variants(self) -> None:
        manifest = load_component_manifest()
        valid_host = OperationReceipt.rejected(
            "op-host-failure", "install", ("core",), (), Rejection.from_failure(RejectionStage.HOST, ComponentResolutionError("invalid-installed-inventory"))
        )
        valid_plan = OperationReceipt.rejected(
            "op-plan-failure", "remove", ("core",), ("core", "github"), Rejection.from_failure(RejectionStage.PLAN, ComponentResolutionError("dependency-protected-removal"))
        )
        invalid = (
            OperationReceipt("op-not-a-command-error", "install", "rejected", ("core",), (), (), (), ("components-not-accepted",)),
            OperationReceipt("op-post-mutation-error", "install", "rejected", ("core",), (), (), (), ("installed-state-mismatch",)),
            OperationReceipt.rejected("op-request-after-host", "install", ("unknown",), ("core",), Rejection.from_failure(RejectionStage.REQUEST, ComponentResolutionError("unknown-component"))),
            OperationReceipt("op-pending", "install", "pending", ("core",), ("core",), (), ("core",), ()),  # type: ignore[arg-type]
            OperationReceipt("op-free-text", "install", "rejected", ("core",), (), (), (), ("host said no",)),
        )

        valid_host.validate(manifest)
        valid_plan.validate(manifest)
        with self.assertRaisesRegex(ValueError, "stage"):
            Rejection.from_failure(RejectionStage.HOST, ComponentResolutionError("unknown-component")).validate(
                manifest, "install", ("unknown",), (), plan_transition
            )
        for receipt in invalid:
            with self.subTest(receipt=receipt.identifier):
                with self.assertRaises(ValueError):
                    receipt.validate(manifest)

    def test_receipt_serialization_has_no_direct_builder_bypass(self) -> None:
        manifest = load_component_manifest()
        plan = plan_transition(manifest, "install", ("core",), (), ())
        journal = plan.journal("op-typed-terminal", InventorySnapshot(None))
        rejected = OperationReceipt.rejected(
            "op-typed-rejected", "remove", (), (), Rejection.from_failure(RejectionStage.REQUEST, ComponentResolutionError("missing-removal-target"))
        )
        invalid = (
            OperationReceipt("op-pending-encode", "install", "pending", (), (), (), (), ()),  # type: ignore[arg-type]
            OperationReceipt("op-unknown-command", "unknown", "completed", (), (), (), (), ()),  # type: ignore[arg-type]
            OperationReceipt("op-unknown-outcome", "install", "unknown", (), (), (), (), ()),  # type: ignore[arg-type]
        )

        self.assertFalse(hasattr(receipt_storage, "operation_receipt"))
        for receipt in (journal.receipt("completed"), journal.receipt("rolled-back", journal.before), rejected):
            receipt.validate(manifest)
            self.assertIsInstance(receipt.encode(), dict)
        for receipt in invalid:
            with self.subTest(receipt=receipt.identifier):
                with self.assertRaises(ValueError):
                    receipt.encode()
        with TemporaryDirectory() as temporary:
            home = Path(temporary)
            receipt_storage.write_receipt(home, manifest, journal.receipt("completed"))
            self.assertEqual(receipt_storage.read_receipt(home, "op-typed-terminal"), journal.receipt("completed").encode())
            with self.assertRaises(TypeError):
                receipt_storage.write_receipt(home, manifest, {})  # type: ignore[arg-type]
        repository = Path(__file__).resolve().parents[3]
        for path in (
            repository / "packages/getcodexy/src/codexy_runtime_tools/component_lifecycle.py",
            repository / "packages/getcodexy/src/codexy_runtime_tools/component_transaction_receipts.py",
        ):
            source = path.read_text(encoding="utf-8")
            self.assertNotIn("operation_receipt(", source)
            self.assertNotIn("OperationReceipt(", source)

    def test_typed_receipts_preserve_only_safe_operation_identities(self) -> None:
        manifest = load_component_manifest()
        plan = plan_transition(manifest, "install", ("core",), (), ())
        valid = plan.journal("op-" + "x" * 128, InventorySnapshot(None)).receipt("completed")
        invalid = ("", "bad", "op-", "op-../escape", "op-" + "x" * 129)

        with TemporaryDirectory() as temporary:
            home = Path(temporary)
            receipt_storage.write_receipt(home, manifest, valid)
            saved = home / "getcodexy" / "receipts" / f"{valid.identifier}.json"
            self.assertEqual(json.loads(saved.read_text(encoding="utf-8"))["operation_id"], valid.identifier)
            for identifier in invalid:
                with self.subTest(identifier=identifier):
                    receipt = OperationReceipt(identifier, "install", "completed", ("core",), ("core",), (), ("core",), ())
                    with self.assertRaises(ValueError):
                        receipt.encode()
                    with self.assertRaises(ValueError):
                        receipt_storage.write_receipt(home, manifest, receipt)
                    self.assertEqual(list((home / "getcodexy" / "receipts").iterdir()), [saved])
                    with self.assertRaises(ValueError):
                        operation_id(identifier)

        valid_journal = plan.journal("op-journal-id", InventorySnapshot(None))
        invalid_journal = Journal("bad", valid_journal.command, valid_journal.requested, valid_journal.resolved, valid_journal.before, valid_journal.target, valid_journal.snapshot, valid_journal.phase)
        with self.assertRaisesRegex(ValueError, "identifiers"):
            invalid_journal.encode()
        with self.assertRaisesRegex(ValueError, "identifiers"):
            invalid_journal.validate(manifest, decode_inventory)
        payload = valid_journal.encode() | {"operation_id": "op-"}
        with self.assertRaisesRegex(ValueError, "identifiers"):
            Journal.decode(payload)

    def test_operation_identifier_boundaries_fail_closed(self) -> None:
        manifest = load_component_manifest()
        plan = plan_transition(manifest, "install", ("core",), (), ())
        invalid = ("", "bad", "op-", "op-../escape", "op-" + "x" * 129)

        generated = Mock(hex="generated")
        with patch("codexy_runtime_tools.component_transaction_identity.uuid.uuid4", return_value=generated) as uuid4:
            self.assertEqual(operation_id(None), "op-generated")
            self.assertEqual(operation_id("op-valid"), "op-valid")
            self.assertEqual(operation_id("op-" + "x" * 128), "op-" + "x" * 128)
            for identifier in invalid:
                with self.subTest(boundary="generator", identifier=identifier):
                    with self.assertRaises(ValueError):
                        operation_id(identifier)
            uuid4.assert_called_once_with()

        journal = plan.journal("op-identity-boundaries", InventorySnapshot(None))
        receipt = journal.receipt("completed").encode()
        with TemporaryDirectory() as temporary:
            home = Path(temporary)
            receipt_storage.write_receipt(home, manifest, journal.receipt("completed"))
            for identifier in invalid:
                with self.subTest(boundary="journal-start", identifier=identifier):
                    with self.assertRaisesRegex(ValueError, "identifiers"):
                        plan.journal(identifier, InventorySnapshot(None))
                invalid_journal = Journal(identifier, journal.command, journal.requested, journal.resolved, journal.before, journal.target, journal.snapshot, journal.phase)
                with self.subTest(boundary="journal-encode", identifier=identifier):
                    with self.assertRaisesRegex(ValueError, "identifiers"):
                        invalid_journal.encode()
                with self.subTest(boundary="journal-validate", identifier=identifier):
                    with self.assertRaisesRegex(ValueError, "identifiers"):
                        invalid_journal.validate(manifest, decode_inventory)
                with self.subTest(boundary="journal-decode", identifier=identifier):
                    with self.assertRaisesRegex(ValueError, "identifiers"):
                        Journal.decode(journal.encode() | {"operation_id": identifier})
                invalid_receipt = OperationReceipt(identifier, "install", "completed", ("core",), ("core",), (), ("core",), ())
                with self.subTest(boundary="receipt-encode", identifier=identifier):
                    with self.assertRaises(ValueError):
                        invalid_receipt.encode()
                with self.subTest(boundary="receipt-validate", identifier=identifier):
                    with self.assertRaises(ValueError):
                        invalid_receipt.validate(manifest)
                with self.subTest(boundary="receipt-persistence", identifier=identifier):
                    with self.assertRaises(ValueError):
                        receipt_storage.write_receipt(home, manifest, invalid_receipt)
                with self.subTest(boundary="receipt-decode", identifier=identifier):
                    with self.assertRaises(ValueError):
                        OperationReceipt.decode(receipt | {"operation_id": identifier})


def _requests() -> tuple[tuple[str, tuple[str, ...]], ...]:
    selections = ((), ("core",), ("github",), ("devtools",), ("core", "github"), ("core", "devtools"), ("core", "github", "devtools"))
    return tuple((command, selected) for command in ("install", "update", "remove") for selected in selections)


def _inventory(selection: tuple[str, ...]) -> bytes:
    return json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": list(selection)}).encode()


def _identifier(command: str, before: tuple[str, ...], requested: tuple[str, ...], recorded: tuple[str, ...] | None) -> str:
    values = (command, "none" if recorded is None else "recorded", *(before or ("empty",)), *(requested or ("all",)))
    return "op-model-" + "-".join(values)


if __name__ == "__main__":
    unittest.main()
