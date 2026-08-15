"""Operation receipt identity and persistence cases."""

import json
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import Mock, patch

from codexy_runtime_tools import component_transaction_receipts as receipt_storage
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_resolver import ComponentResolutionError
from codexy_runtime_tools.component_transaction_identity import operation_id
from codexy_runtime_tools.component_transaction_state import decode_inventory
from codexy_runtime_tools.component_transition_model import (
    InventorySnapshot,
    Journal,
    OperationReceipt,
    plan_transition,
)
from codexy_runtime_tools.component_transition_rejections import (
    Rejection,
    RejectionStage,
)


class ComponentReceiptIdentityCases:
    def test_receipt_serialization_has_no_direct_builder_bypass(self) -> None:
        manifest = load_component_manifest()
        plan = plan_transition(manifest, "install", ("core",), (), ())
        journal = plan.journal("op-typed-terminal", InventorySnapshot(None))
        rejected = OperationReceipt.rejected(
            "op-typed-rejected",
            "remove",
            (),
            (),
            Rejection.from_failure(
                RejectionStage.REQUEST,
                ComponentResolutionError("missing-removal-target"),
            ),
        )
        invalid = (
            OperationReceipt(
                "op-pending-encode", "install", "pending", (), (), (), (), ()
            ),  # type: ignore[arg-type]
            OperationReceipt(
                "op-unknown-command", "unknown", "completed", (), (), (), (), ()
            ),  # type: ignore[arg-type]
            OperationReceipt(
                "op-unknown-outcome", "install", "unknown", (), (), (), (), ()
            ),  # type: ignore[arg-type]
        )

        self.assertFalse(hasattr(receipt_storage, "operation_receipt"))
        for receipt in (
            journal.receipt("completed"),
            journal.receipt("rolled-back", journal.before),
            rejected,
        ):
            receipt.validate(manifest)
            self.assertIsInstance(receipt.encode(), dict)
        for receipt in invalid:
            with self.subTest(receipt=receipt.identifier):
                with self.assertRaises(ValueError):
                    receipt.encode()
        with TemporaryDirectory() as temporary:
            home = Path(temporary)
            receipt_storage.write_receipt(home, manifest, journal.receipt("completed"))
            self.assertEqual(
                receipt_storage.read_receipt(home, "op-typed-terminal"),
                journal.receipt("completed").encode(),
            )
            with self.assertRaises(TypeError):
                receipt_storage.write_receipt(home, manifest, {})  # type: ignore[arg-type]
        repository = Path(__file__).resolve().parents[3]
        for path in (
            repository
            / "packages/getcodexy/src/codexy_runtime_tools/component_lifecycle.py",
            repository
            / "packages/getcodexy/src/codexy_runtime_tools/component_transaction_receipts.py",
        ):
            source = path.read_text(encoding="utf-8")
            self.assertNotIn("operation_receipt(", source)
            self.assertNotIn("OperationReceipt(", source)

    def test_typed_receipts_preserve_only_safe_operation_identities(self) -> None:
        manifest = load_component_manifest()
        plan = plan_transition(manifest, "install", ("core",), (), ())
        valid = plan.journal("op-" + "x" * 128, InventorySnapshot(None)).receipt(
            "completed"
        )
        invalid = ("", "bad", "op-", "op-../escape", "op-" + "x" * 129)

        with TemporaryDirectory() as temporary:
            home = Path(temporary)
            receipt_storage.write_receipt(home, manifest, valid)
            saved = home / "getcodexy" / "receipts" / f"{valid.identifier}.json"
            self.assertEqual(
                json.loads(saved.read_text(encoding="utf-8"))["operation_id"],
                valid.identifier,
            )
            for identifier in invalid:
                with self.subTest(identifier=identifier):
                    receipt = OperationReceipt(
                        identifier,
                        "install",
                        "completed",
                        ("core",),
                        ("core",),
                        (),
                        ("core",),
                        (),
                    )
                    with self.assertRaises(ValueError):
                        receipt.encode()
                    with self.assertRaises(ValueError):
                        receipt_storage.write_receipt(home, manifest, receipt)
                    self.assertEqual(
                        list((home / "getcodexy" / "receipts").iterdir()), [saved]
                    )
                    with self.assertRaises(ValueError):
                        operation_id(identifier)

        valid_journal = plan.journal("op-journal-id", InventorySnapshot(None))
        invalid_journal = Journal(
            "bad",
            valid_journal.command,
            valid_journal.requested,
            valid_journal.resolved,
            valid_journal.before,
            valid_journal.target,
            valid_journal.snapshot,
            valid_journal.phase,
        )
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
        with patch(
            "codexy_runtime_tools.component_transaction_identity.uuid.uuid4",
            return_value=generated,
        ) as uuid4:
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
                invalid_journal = Journal(
                    identifier,
                    journal.command,
                    journal.requested,
                    journal.resolved,
                    journal.before,
                    journal.target,
                    journal.snapshot,
                    journal.phase,
                )
                with self.subTest(boundary="journal-encode", identifier=identifier):
                    with self.assertRaisesRegex(ValueError, "identifiers"):
                        invalid_journal.encode()
                with self.subTest(boundary="journal-validate", identifier=identifier):
                    with self.assertRaisesRegex(ValueError, "identifiers"):
                        invalid_journal.validate(manifest, decode_inventory)
                with self.subTest(boundary="journal-decode", identifier=identifier):
                    with self.assertRaisesRegex(ValueError, "identifiers"):
                        Journal.decode(journal.encode() | {"operation_id": identifier})
                invalid_receipt = OperationReceipt(
                    identifier,
                    "install",
                    "completed",
                    ("core",),
                    ("core",),
                    (),
                    ("core",),
                    (),
                )
                with self.subTest(boundary="receipt-encode", identifier=identifier):
                    with self.assertRaises(ValueError):
                        invalid_receipt.encode()
                with self.subTest(boundary="receipt-validate", identifier=identifier):
                    with self.assertRaises(ValueError):
                        invalid_receipt.validate(manifest)
                with self.subTest(
                    boundary="receipt-persistence", identifier=identifier
                ):
                    with self.assertRaises(ValueError):
                        receipt_storage.write_receipt(home, manifest, invalid_receipt)
                with self.subTest(boundary="receipt-decode", identifier=identifier):
                    with self.assertRaises(ValueError):
                        OperationReceipt.decode(receipt | {"operation_id": identifier})
