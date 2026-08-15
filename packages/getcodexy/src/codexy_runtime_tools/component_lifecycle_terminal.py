"""Terminal receipt encoding and rejection helpers for lifecycle operations."""

from __future__ import annotations

from pathlib import Path

from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError
from .component_transaction_receipts import write_receipt
from .component_transition_model import (
    OperationReceipt,
    Rejection,
    RejectionStage,
    StateFailure,
    plan_transition,
)


def terminal(
    home: Path, manifest: ComponentManifest, receipt: OperationReceipt
) -> dict[str, object]:
    receipt.validate(manifest)
    encoded = receipt.encode()
    write_receipt(home, manifest, receipt)
    return encoded


def reject(
    home: Path,
    manifest: ComponentManifest,
    identifier: str,
    command: str,
    requested: tuple[str, ...],
    before: tuple[str, ...],
    stage: RejectionStage,
    failure: ComponentResolutionError | StateFailure,
) -> dict[str, object]:
    rejection = Rejection.from_failure(stage, failure)
    rejection.validate(manifest, command, requested, before, plan_transition)
    return terminal(
        home,
        manifest,
        OperationReceipt.rejected(identifier, command, requested, before, rejection),
    )  # type: ignore[arg-type]
