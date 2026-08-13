"""Closed domain model for durable component lifecycle transitions."""

from __future__ import annotations

import base64
from dataclasses import dataclass
from enum import Enum
from typing import Callable, Literal

from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError, canonical_components, resolve_components
from .component_transaction_identity import operation_id


Command = Literal["install", "update", "remove"]
Phase = Literal["started", "rolling-back", "committed"]
Outcome = Literal["completed", "rejected", "rolled-back"]
JOURNAL_SCHEMA = "getcodexy.component-transaction.v1"
RECEIPT_SCHEMA = "getcodexy.operation-receipt.v1"
SOURCE = "installed-component-inventory"


class StateFailure(str, Enum):
    INCONSISTENT_INSTALLED_STATE = "inconsistent-installed-state"


class PreStateSource(str, Enum):
    DURABLE_SNAPSHOT = "durable-snapshot"
    NO_SNAPSHOT = "no-snapshot"


HOST_REJECTION_CODES = frozenset({
    "component-version-mismatch", "components-not-accepted", "conflicting-installed-state",
    "inconsistent-installed-state", "installed-state-mismatch", "invalid-installed-inventory",
    "mixed-version-state", "unknown-installed-component",
})


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


@dataclass(frozen=True)
class Journal:
    identifier: str
    command: Command
    requested: tuple[str, ...]
    resolved: tuple[str, ...]
    before: tuple[str, ...]
    target: tuple[str, ...]
    snapshot: InventorySnapshot
    phase: Phase

    @classmethod
    def start(cls, identifier: str, plan: "TransitionPlan", snapshot: InventorySnapshot) -> "Journal":
        journal = cls(identifier, plan.command, plan.requested, plan.resolved, plan.before, plan.target, snapshot, "started")
        journal._require_snapshot()
        return journal

    @classmethod
    def decode(cls, value: object) -> "Journal":
        fields = {"schema", "operation_id", "command", "requested", "resolved", "before", "target", "inventory", "phase"}
        if not isinstance(value, dict) or set(value) != fields or value.get("schema") != JOURNAL_SCHEMA:
            raise ValueError("component transaction journal has an invalid shape")
        command, phase = value.get("command"), value.get("phase")
        components = tuple(_components(value, field, "component transaction journal") for field in ("requested", "resolved", "before", "target"))
        identifier, encoded = value.get("operation_id"), value.get("inventory")
        if command not in {"install", "update", "remove"} or phase not in {"started", "rolling-back", "committed"} or not isinstance(identifier, str) or operation_id(identifier) != identifier or not isinstance(encoded, str):
            raise ValueError("component transaction journal has invalid identifiers")
        try:
            snapshot = InventorySnapshot(base64.b64decode(encoded.encode(), validate=True) or None)
        except ValueError as error:
            raise ValueError("component transaction journal has invalid inventory") from error
        journal = cls(identifier, command, *components, snapshot, phase)
        journal._require_snapshot()
        return journal

    def encode(self) -> dict[str, object]:
        return {"schema": JOURNAL_SCHEMA, "operation_id": self.identifier, "command": self.command, "requested": list(self.requested), "resolved": list(self.resolved), "before": list(self.before), "target": list(self.target), "inventory": base64.b64encode(self.snapshot.contents or b"").decode(), "phase": self.phase}

    def with_phase(self, phase: Phase) -> "Journal":
        if phase not in {"started", "rolling-back", "committed"}:
            raise ValueError("component transaction journal has an invalid phase")
        return Journal(self.identifier, self.command, self.requested, self.resolved, self.before, self.target, self.snapshot, phase)

    def validate(self, manifest: ComponentManifest, decode_snapshot: Callable[[bytes], tuple[str, ...]]) -> None:
        self._require_snapshot()
        if any(value != canonical_components(manifest, set(value)) for value in (self.before, self.target, self.resolved)) or self.before not in manifest.compatible_combinations or self.target not in manifest.compatible_combinations:
            raise ValueError("component transaction journal is inconsistent")
        if self.snapshot.contents is not None and decode_snapshot(self.snapshot.contents) != self.before:
            raise ValueError("component transaction journal does not match its inventory snapshot")
        try:
            plan = plan_transition(manifest, self.command, self.requested, self.before, self.before)
        except ComponentResolutionError as error:
            raise ValueError("component transaction journal has an invalid request") from error
        if (plan.resolved, plan.target) != (self.resolved, self.target):
            raise ValueError("component transaction journal does not match its plan")

    def receipt(self, outcome: Outcome, after: tuple[str, ...] | None = None) -> "OperationReceipt":
        return OperationReceipt.from_journal(self, outcome, self.target if after is None else after)

    @property
    def pre_state_source(self) -> PreStateSource:
        return PreStateSource.NO_SNAPSHOT if self.snapshot.contents is None else PreStateSource.DURABLE_SNAPSHOT

    def _require_snapshot(self) -> None:
        if self.command == "update" and self.pre_state_source is PreStateSource.NO_SNAPSHOT:
            raise ValueError("component transaction update journal requires an inventory snapshot")


@dataclass(frozen=True)
class TransitionPlan:
    command: Command
    requested: tuple[str, ...]
    before: tuple[str, ...]
    resolved: tuple[str, ...]
    target: tuple[str, ...]
    adds: tuple[str, ...]
    removes: tuple[str, ...]

    def journal(self, identifier: str, snapshot: InventorySnapshot) -> Journal:
        return Journal.start(identifier, self, snapshot)


@dataclass(frozen=True)
class OperationReceipt:
    identifier: str
    command: Command
    outcome: Outcome
    requested: tuple[str, ...]
    resolved: tuple[str, ...]
    before: tuple[str, ...]
    after: tuple[str, ...]
    errors: tuple[str, ...]

    @classmethod
    def rejected(cls, identifier: str, command: Command, requested: tuple[str, ...], before: tuple[str, ...], failure: ComponentResolutionError | StateFailure) -> "OperationReceipt":
        return cls(identifier, command, "rejected", requested, (), before, before, (failure.code if isinstance(failure, ComponentResolutionError) else failure.value,))

    @classmethod
    def from_journal(cls, journal: Journal, outcome: Outcome, after: tuple[str, ...]) -> "OperationReceipt":
        if outcome == "rejected":
            raise ValueError("a journal cannot produce a rejected receipt")
        if outcome == "completed" and after != journal.target:
            raise ValueError("a completion receipt must use the transition target")
        if outcome == "rolled-back" and after != journal.before:
            raise ValueError("a rollback receipt must use the transition pre-state")
        errors = () if outcome == "completed" else ("operation-failed",)
        return cls(journal.identifier, journal.command, outcome, journal.requested, journal.resolved, journal.before, after, errors)

    @classmethod
    def decode(cls, value: object) -> "OperationReceipt":
        fields = {"schema", "operation_id", "command", "outcome", "requested_components", "resolved_components", "selection_before", "selection_after", "installed_components", "source_of_truth", "errors"}
        if not isinstance(value, dict) or set(value) != fields or value.get("schema") != RECEIPT_SCHEMA or value.get("source_of_truth") != SOURCE or value.get("installed_components") != value.get("selection_after"):
            raise ValueError("operation receipt has an invalid shape")
        identifier, command, outcome = value.get("operation_id"), value.get("command"), value.get("outcome")
        if not isinstance(identifier, str) or operation_id(identifier) != identifier or command not in {"install", "update", "remove"} or outcome not in {"completed", "rejected", "rolled-back"}:
            raise ValueError("operation receipt has an invalid shape")
        parts = tuple(_components(value, field, "operation receipt") for field in ("requested_components", "resolved_components", "selection_before", "selection_after"))
        errors = value.get("errors")
        if not isinstance(errors, list) or any(not isinstance(error, dict) or set(error) != {"code"} or not isinstance(error.get("code"), str) for error in errors):
            raise ValueError("operation receipt has an invalid shape")
        return cls(identifier, command, outcome, *parts, tuple(error["code"] for error in errors))

    def encode(self) -> dict[str, object]:
        return {"schema": RECEIPT_SCHEMA, "operation_id": self.identifier, "command": self.command, "outcome": self.outcome, "requested_components": list(self.requested), "resolved_components": list(self.resolved), "selection_before": list(self.before), "selection_after": list(self.after), "installed_components": list(self.after), "source_of_truth": SOURCE, "errors": [{"code": error} for error in self.errors]}

    def validate(self, manifest: ComponentManifest) -> None:
        if self.before not in manifest.compatible_combinations or self.after not in manifest.compatible_combinations:
            raise ValueError("operation receipt has invalid component selections")
        if self.outcome == "rejected":
            if self.resolved or self.after != self.before or not _valid_rejection(manifest, self.command, self.requested, self.before, self.errors):
                raise ValueError("operation receipt has invalid rejection semantics")
            return
        try:
            plan = plan_transition(manifest, self.command, self.requested, self.before, self.before)
        except ComponentResolutionError as error:
            raise ValueError("operation receipt has an invalid request contract") from error
        if self.resolved != plan.resolved:
            raise ValueError("operation receipt has an invalid resolved selection")
        if self.outcome == "completed" and (self.after != plan.target or self.errors):
            raise ValueError("operation receipt has invalid completion semantics")
        if self.outcome == "rolled-back" and (self.after != self.before or self.errors != ("operation-failed",)):
            raise ValueError("operation receipt has invalid rollback semantics")


def plan_transition(manifest: ComponentManifest, command: Command, requested: tuple[str, ...], before: tuple[str, ...], recorded: tuple[str, ...] | None) -> TransitionPlan:
    if command == "install":
        resolved = resolve_components(manifest, requested)
        target = canonical_components(manifest, set(before) | set(resolved))
        return TransitionPlan(command, requested, before, resolved, target, canonical_components(manifest, set(target) - set(before)), ())
    if command == "update":
        if recorded is None:
            raise ComponentResolutionError("no-recorded-selection")
        resolved = before if not requested else resolve_components(manifest, requested)
        if not set(resolved).issubset(before):
            raise ComponentResolutionError("incompatible-component-selection")
        return TransitionPlan(command, requested, before, resolved, before, resolved, ())
    if not requested:
        raise ComponentResolutionError("missing-removal-target")
    resolve_components(manifest, requested)
    resolved = canonical_components(manifest, set(requested))
    target = canonical_components(manifest, set(before) - set(resolved))
    if target not in manifest.compatible_combinations:
        raise ComponentResolutionError("dependency-protected-removal")
    return TransitionPlan(command, requested, before, resolved, target, (), tuple(reversed(resolved)))


def _components(value: dict[str, object], field: str, subject: str) -> tuple[str, ...]:
    components = value.get(field)
    if not isinstance(components, list) or any(not isinstance(component, str) for component in components):
        raise ValueError(f"{subject} has invalid components")
    return tuple(components)


def _valid_rejection(manifest: ComponentManifest, command: Command, requested: tuple[str, ...], before: tuple[str, ...], errors: tuple[str, ...]) -> bool:
    return len(errors) == 1 and errors[0] in _rejection_codes(manifest, command, requested, before)


def _rejection_codes(manifest: ComponentManifest, command: Command, requested: tuple[str, ...], before: tuple[str, ...]) -> frozenset[str]:
    """All rejected paths represent either host admission or a failed pure plan."""
    codes = set(HOST_REJECTION_CODES)
    for recorded in (None, before):
        try:
            plan_transition(manifest, command, requested, before, recorded)
        except ComponentResolutionError as error:
            codes.add(error.code)
    return frozenset(codes)
