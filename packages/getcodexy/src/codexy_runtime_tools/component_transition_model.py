"""Public transition planning surface for component lifecycle operations."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from .component_manifest import ComponentManifest
from .component_resolver import (
    ComponentResolutionError,
    canonical_components,
    resolve_components,
)
from .component_transition_journal import Journal, PreStateSource
from .component_transition_journal import JOURNAL_SCHEMA
from .component_transition_receipt import OperationReceipt, RECEIPT_SCHEMA, SOURCE
from .component_transition_rejections import (
    Rejection,
    RejectionKind,
    RejectionStage,
    StateFailure,
)
from .component_transaction_snapshot import InventorySnapshot

Command = Literal["install", "update", "remove", "bootstrap"]
Phase = Literal["started", "rolling-back", "committed"]
Outcome = Literal["completed", "rejected", "rolled-back"]


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


def plan_transition(
    manifest: ComponentManifest,
    command: Command,
    requested: tuple[str, ...],
    before: tuple[str, ...],
    recorded: tuple[str, ...] | None,
) -> TransitionPlan:
    if command == "remove":
        if not requested:
            raise ComponentResolutionError("missing-removal-target")
        resolve_components(manifest, requested)
        resolved = canonical_components(manifest, set(requested))
        target = canonical_components(manifest, set(before) - set(resolved))
        if target not in manifest.compatible_combinations:
            raise ComponentResolutionError("dependency-protected-removal")
        return TransitionPlan(
            command, requested, before, resolved, target, (), tuple(reversed(resolved))
        )

    if command == "bootstrap":
        if requested:
            raise ComponentResolutionError("components-not-accepted")
        reconciliation_request = ()
    elif command == "update":
        if recorded is None:
            raise ComponentResolutionError("no-recorded-selection")
        reconciliation_request = requested or before
    else:
        reconciliation_request = requested

    resolved = resolve_components(manifest, reconciliation_request)
    if command == "update" and not set(resolved).issubset(before):
        raise ComponentResolutionError("incompatible-component-selection")
    target = (
        before
        if command == "update"
        else canonical_components(manifest, set(before) | set(resolved))
    )
    adds = (
        resolved
        if command in {"update", "bootstrap"}
        else canonical_components(manifest, set(target) - set(before))
    )
    return TransitionPlan(command, requested, before, resolved, target, adds, ())


__all__ = [
    "Command",
    "Journal",
    "JOURNAL_SCHEMA",
    "OperationReceipt",
    "Outcome",
    "Phase",
    "PreStateSource",
    "Rejection",
    "RejectionKind",
    "RejectionStage",
    "RECEIPT_SCHEMA",
    "SOURCE",
    "StateFailure",
    "TransitionPlan",
    "plan_transition",
]
