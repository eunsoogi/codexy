"""Opaque durable local-inventory snapshot used by lifecycle transitions."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class InventorySnapshot:
    contents: bytes | None

    @classmethod
    def capture(cls, home: object) -> "InventorySnapshot":
        from .component_transaction_state import capture_inventory_snapshot

        return capture_inventory_snapshot(home)

    def restore(self, home: object) -> None:
        from .component_transaction_state import restore_inventory_snapshot

        restore_inventory_snapshot(home, self)
