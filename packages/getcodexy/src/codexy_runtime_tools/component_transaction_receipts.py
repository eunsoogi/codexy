"""Strict immutable operation receipt persistence and lookup."""

from __future__ import annotations

import json
from pathlib import Path

from .component_manifest import ComponentManifest
from .component_transition_model import OperationReceipt, RECEIPT_SCHEMA, SOURCE
from .component_transaction_state import (
    _atomic_write,
    _read_regular,
    inventory_path,
    _unique_object,
)


def read_receipt(home: Path, identifier: str) -> dict[str, object] | None:
    contents = _read_regular(_receipt_path(home, identifier))
    if contents is None:
        return None
    value = json.loads(contents, object_pairs_hook=_unique_object)
    try:
        return OperationReceipt.decode(value).encode()
    except ValueError:
        raise ValueError("operation receipt has an invalid shape")


def write_receipt(
    home: Path, manifest: ComponentManifest, receipt: OperationReceipt
) -> None:
    if not isinstance(receipt, OperationReceipt):
        raise TypeError(
            "operation receipt persistence requires a typed terminal receipt"
        )
    receipt.validate(manifest)
    identifier = receipt.identifier
    target, contents = (
        _receipt_path(home, identifier),
        json.dumps(receipt.encode(), sort_keys=True).encode(),
    )
    if existing := _read_regular(target):
        if existing != contents:
            raise ValueError(f"operation receipt already exists: {identifier}")
        return
    _atomic_write(target, contents)


def _receipt_path(home: Path, identifier: str) -> Path:
    return inventory_path(home).parent / "receipts" / f"{identifier}.json"
