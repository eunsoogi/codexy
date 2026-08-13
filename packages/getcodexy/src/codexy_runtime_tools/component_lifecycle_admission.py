"""Pre-mutation inventory and operation receipt admission for lifecycle commands."""

from __future__ import annotations

from pathlib import Path

from .component_manifest import ComponentManifest
from .component_resolver import admit_installed_inventory, admit_recovery_inventory
from .component_transaction_receipts import read_receipt
from .component_transaction_state import Journal
from .component_transition_model import OperationReceipt


def admitted_selection(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None) -> tuple[str, ...]:
    return admit_installed_inventory(manifest, inventory, marketplace_root)


def admitted_recovery_selection(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None, expected: tuple[str, ...]) -> tuple[str, ...]:
    return admit_recovery_inventory(manifest, inventory, marketplace_root, expected)


def replay_receipt(home: Path, manifest: ComponentManifest, identifier: str, command: str, requested: tuple[str, ...]) -> dict[str, object] | None:
    receipt = read_receipt(home, identifier)
    if receipt is None:
        return None
    _validate_receipt(manifest, receipt)
    if receipt["operation_id"] == identifier and receipt["command"] == command and receipt["requested_components"] == list(requested):
        return receipt
    raise ValueError(f"operation receipt conflicts with requested operation: {identifier}")


def admit_pending_receipt(home: Path, manifest: ComponentManifest, journal: Journal) -> dict[str, object] | None:
    receipt = read_receipt(home, journal.identifier)
    if receipt is None:
        return None
    _validate_receipt(manifest, receipt)
    if receipt["operation_id"] != journal.identifier or receipt["command"] != journal.command or receipt["requested_components"] != list(journal.requested) or receipt["resolved_components"] != list(journal.resolved) or receipt["selection_before"] != list(journal.before):
        raise ValueError(f"pending transaction receipt conflicts with journal: {journal.identifier}")
    if journal.phase == "committed" and receipt["outcome"] == "completed" and receipt["selection_after"] == list(journal.target):
        return receipt
    if journal.phase == "rolling-back" and receipt["outcome"] == "rolled-back" and receipt["selection_after"] == list(journal.before):
        return receipt
    raise ValueError(f"pending transaction receipt conflicts with journal: {journal.identifier}")


def matching_receipt(home: Path, manifest: ComponentManifest, receipt: dict[str, object]) -> bool:
    identifier = receipt["operation_id"]
    existing = read_receipt(home, identifier) if isinstance(identifier, str) else None
    if existing is not None:
        _validate_receipt(manifest, existing)
    return existing == receipt


def _validate_receipt(manifest: ComponentManifest, receipt: dict[str, object]) -> None:
    try:
        OperationReceipt.decode(receipt).validate(manifest)
    except ValueError as error:
        if str(error).startswith("operation receipt"):
            raise
        raise ValueError(f"operation receipt {error}") from error
