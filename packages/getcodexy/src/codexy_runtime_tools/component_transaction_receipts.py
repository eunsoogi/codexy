"""Strict immutable operation receipt persistence and lookup."""

from __future__ import annotations

import json
from pathlib import Path

from .component_transaction_identity import operation_id
from .component_transaction_state import _atomic_write, _read_regular, inventory_path, _unique_object


def read_receipt(home: Path, identifier: str) -> dict[str, object] | None:
    contents = _read_regular(_receipt_path(home, identifier))
    if contents is None:
        return None
    value = json.loads(contents, object_pairs_hook=_unique_object)
    if not _valid_receipt(value):
        raise ValueError("operation receipt has an invalid shape")
    return value


def write_receipt(home: Path, receipt: dict[str, object]) -> None:
    identifier = operation_id(receipt.get("operation_id") if isinstance(receipt.get("operation_id"), str) else None)
    target, contents = _receipt_path(home, identifier), json.dumps(receipt, sort_keys=True).encode()
    if existing := _read_regular(target):
        if existing != contents:
            raise ValueError(f"operation receipt already exists: {identifier}")
        return
    _atomic_write(target, contents)


def _receipt_path(home: Path, identifier: str) -> Path:
    return inventory_path(home).parent / "receipts" / f"{identifier}.json"


def _valid_receipt(value: object) -> bool:
    fields = {"schema", "operation_id", "command", "outcome", "requested_components", "resolved_components", "selection_before", "selection_after", "installed_components", "source_of_truth", "errors"}
    lists = ("requested_components", "resolved_components", "selection_before", "selection_after", "installed_components")
    if not isinstance(value, dict) or set(value) != fields or not all(isinstance(value.get(name), list) and all(isinstance(item, str) for item in value[name]) for name in lists):
        return False
    identifier = value.get("operation_id")
    return (
        isinstance(identifier, str)
        and operation_id(identifier) == identifier
        and isinstance(value.get("command"), str)
        and value.get("outcome") in {"completed", "rejected", "rolled-back"}
        and value.get("source_of_truth") == "installed-component-inventory"
        and isinstance(value.get("errors"), list)
    )
