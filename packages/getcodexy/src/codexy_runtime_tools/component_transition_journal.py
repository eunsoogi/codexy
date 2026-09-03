"""Durable journal encoding and validation for component transitions."""

from __future__ import annotations

import base64
from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING, Callable

from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError, canonical_components
from .component_transaction_identity import valid_operation_id
from .component_transaction_snapshot import InventorySnapshot

if TYPE_CHECKING:
    from .component_transition_model import Outcome, TransitionPlan
    from .component_transition_receipt import OperationReceipt

JOURNAL_SCHEMA = "getcodexy.component-transaction.v1"


class PreStateSource(str, Enum):
    DURABLE_SNAPSHOT = "durable-snapshot"
    NO_SNAPSHOT = "no-snapshot"


@dataclass(frozen=True)
class Journal:
    identifier: str
    command: str
    requested: tuple[str, ...]
    resolved: tuple[str, ...]
    before: tuple[str, ...]
    target: tuple[str, ...]
    snapshot: InventorySnapshot
    phase: str

    @classmethod
    def start(
        cls, identifier: str, plan: TransitionPlan, snapshot: InventorySnapshot
    ) -> Journal:
        if not valid_operation_id(identifier):
            raise ValueError("component transaction journal has invalid identifiers")
        journal = cls(
            identifier,
            plan.command,
            plan.requested,
            plan.resolved,
            plan.before,
            plan.target,
            snapshot,
            "started",
        )
        journal._require_snapshot()
        return journal

    @classmethod
    def decode(cls, value: object) -> Journal:
        fields = {
            "schema",
            "operation_id",
            "command",
            "requested",
            "resolved",
            "before",
            "target",
            "inventory",
            "phase",
        }
        if (
            not isinstance(value, dict)
            or set(value) != fields
            or value.get("schema") != JOURNAL_SCHEMA
        ):
            raise ValueError("component transaction journal has an invalid shape")
        command, phase = value.get("command"), value.get("phase")
        components = tuple(
            _components(value, field, "component transaction journal")
            for field in ("requested", "resolved", "before", "target")
        )
        identifier, encoded = value.get("operation_id"), value.get("inventory")
        if (
            command not in {"install", "update", "remove", "bootstrap"}
            or phase not in {"started", "rolling-back", "committed"}
            or not valid_operation_id(identifier)
            or not isinstance(encoded, str)
        ):
            raise ValueError("component transaction journal has invalid identifiers")
        try:
            snapshot = InventorySnapshot(
                base64.b64decode(encoded.encode(), validate=True) or None
            )
        except ValueError as error:
            raise ValueError(
                "component transaction journal has invalid inventory"
            ) from error
        journal = cls(identifier, command, *components, snapshot, phase)
        journal._require_snapshot()
        return journal

    def encode(self) -> dict[str, object]:
        if not valid_operation_id(self.identifier):
            raise ValueError("component transaction journal has invalid identifiers")
        return {
            "schema": JOURNAL_SCHEMA,
            "operation_id": self.identifier,
            "command": self.command,
            "requested": list(self.requested),
            "resolved": list(self.resolved),
            "before": list(self.before),
            "target": list(self.target),
            "inventory": base64.b64encode(self.snapshot.contents or b"").decode(),
            "phase": self.phase,
        }

    def with_phase(self, phase: str) -> Journal:
        if phase not in {"started", "rolling-back", "committed"}:
            raise ValueError("component transaction journal has an invalid phase")
        return Journal(
            self.identifier,
            self.command,
            self.requested,
            self.resolved,
            self.before,
            self.target,
            self.snapshot,
            phase,
        )

    def validate(
        self,
        manifest: ComponentManifest,
        decode_snapshot: Callable[[bytes], tuple[str, ...]],
    ) -> None:
        if not valid_operation_id(self.identifier):
            raise ValueError("component transaction journal has invalid identifiers")
        self._require_snapshot()
        if (
            any(
                value != canonical_components(manifest, set(value))
                for value in (self.before, self.target, self.resolved)
            )
            or self.before not in manifest.compatible_combinations
            or self.target not in manifest.compatible_combinations
        ):
            raise ValueError("component transaction journal is inconsistent")
        if self.snapshot.contents is not None:
            durable = decode_snapshot(self.snapshot.contents)
            if (
                durable != canonical_components(manifest, set(durable))
                or durable not in manifest.compatible_combinations
            ):
                raise ValueError(
                    "component transaction journal has an invalid durable inventory snapshot"
                )
            if durable != self.before and self.command != "bootstrap":
                raise ValueError(
                    "component transaction journal does not match its inventory snapshot"
                )
        from .component_transition_model import plan_transition

        try:
            plan = plan_transition(
                manifest, self.command, self.requested, self.before, self.before
            )
        except ComponentResolutionError as error:
            raise ValueError(
                "component transaction journal has an invalid request"
            ) from error
        if (plan.resolved, plan.target) != (self.resolved, self.target):
            raise ValueError("component transaction journal does not match its plan")

    def receipt(
        self,
        outcome: Outcome,
        after: tuple[str, ...] | None = None,
        errors: tuple[str, ...] = (),
    ) -> OperationReceipt:
        from .component_transition_receipt import OperationReceipt

        return OperationReceipt.from_journal(
            self, outcome, self.target if after is None else after, errors
        )

    @property
    def pre_state_source(self) -> PreStateSource:
        return (
            PreStateSource.NO_SNAPSHOT
            if self.snapshot.contents is None
            else PreStateSource.DURABLE_SNAPSHOT
        )

    def _require_snapshot(self) -> None:
        if (
            self.command == "update"
            and self.pre_state_source is PreStateSource.NO_SNAPSHOT
        ):
            raise ValueError(
                "component transaction update journal requires an inventory snapshot"
            )


def _components(value: dict[str, object], field: str, subject: str) -> tuple[str, ...]:
    components = value.get(field)
    if not isinstance(components, list) or any(
        not isinstance(component, str) for component in components
    ):
        raise ValueError(f"{subject} has invalid components")
    return tuple(components)
