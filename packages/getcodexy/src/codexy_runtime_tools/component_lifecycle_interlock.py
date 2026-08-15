"""Cross-transaction admission interlocks for component lifecycle operations."""

from __future__ import annotations

from pathlib import Path

from .component_lifecycle_terminal import reject
from .component_manifest import ComponentManifest
from .component_transition_model import RejectionStage, StateFailure
from .monolith_migration_state import read_journal as read_migration_journal


def migration_rejection(
    home: Path,
    manifest: ComponentManifest,
    identifier: str,
    command: str,
    requested: tuple[str, ...],
    lock_held: bool,
) -> dict[str, object] | None:
    if lock_held:
        return None
    try:
        pending = read_migration_journal(home) is not None
    except (OSError, ValueError):
        pending = True
    if not pending:
        return None
    return reject(
        home,
        manifest,
        identifier,
        command,
        requested,
        (),
        RejectionStage.PRESTATE,
        StateFailure.INCONSISTENT_INSTALLED_STATE,
    )
