"""Pre-mutation inventory and operation receipt admission for lifecycle commands."""

from __future__ import annotations

from pathlib import Path

from .component_manifest import ComponentManifest
from .component_resolver import admit_installed_inventory
from .component_transaction_receipts import read_receipt


def admitted_selection(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None) -> tuple[str, ...]:
    return admit_installed_inventory(manifest, inventory, marketplace_root)


def replay_receipt(home: Path, identifier: str, command: str, requested: tuple[str, ...]) -> dict[str, object] | None:
    receipt = read_receipt(home, identifier)
    if receipt is None:
        return None
    if receipt.get("schema") == "getcodexy.operation-receipt.v1" and receipt.get("operation_id") == identifier and receipt.get("command") == command and receipt.get("requested_components") == list(requested) and receipt.get("outcome") in {"completed", "rejected", "rolled-back"}:
        return receipt
    raise ValueError(f"operation receipt conflicts with requested operation: {identifier}")


def matching_receipt(home: Path, receipt: dict[str, object]) -> bool:
    identifier = receipt["operation_id"]
    return isinstance(identifier, str) and read_receipt(home, identifier) == receipt
