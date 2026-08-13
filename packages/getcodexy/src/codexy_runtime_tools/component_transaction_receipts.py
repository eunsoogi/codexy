"""Strict immutable operation receipt persistence and lookup."""

from __future__ import annotations

import json
from pathlib import Path

from .component_transition_model import OperationReceipt, RECEIPT_SCHEMA, SOURCE
from .component_transaction_identity import operation_id
from .component_transaction_state import _atomic_write, _read_regular, inventory_path, _unique_object


def read_receipt(home: Path, identifier: str) -> dict[str, object] | None:
    contents = _read_regular(_receipt_path(home, identifier))
    if contents is None:
        return None
    value = json.loads(contents, object_pairs_hook=_unique_object)
    try:
        return OperationReceipt.decode(value).encode()
    except ValueError:
        raise ValueError("operation receipt has an invalid shape")


def write_receipt(home: Path, receipt: dict[str, object]) -> None:
    decoded = OperationReceipt.decode(receipt)
    identifier = operation_id(decoded.identifier)
    target, contents = _receipt_path(home, identifier), json.dumps(decoded.encode(), sort_keys=True).encode()
    if existing := _read_regular(target):
        if existing != contents:
            raise ValueError(f"operation receipt already exists: {identifier}")
        return
    _atomic_write(target, contents)


def operation_receipt(identifier: str, command: str, requested: tuple[str, ...], resolved: tuple[str, ...], before: tuple[str, ...], after: tuple[str, ...], outcome: str, error: str | None = None) -> dict[str, object]:
    return OperationReceipt(identifier, command, outcome, requested, resolved, before, after, () if error is None else (error,)).encode()  # type: ignore[arg-type]


def _receipt_path(home: Path, identifier: str) -> Path:
    return inventory_path(home).parent / "receipts" / f"{identifier}.json"


def _valid_receipt(value: object) -> bool:
    try:
        OperationReceipt.decode(value)
    except ValueError:
        return False
    return True
