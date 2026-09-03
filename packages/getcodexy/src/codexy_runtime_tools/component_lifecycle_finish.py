"""Finalize committed component operations with host activation readback."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Callable

from .component_hook_activation import HookLister, activation_for_inventory
from .component_lifecycle_admission import (
    admit_pending_receipt,
    matching_receipt,
    replay_receipt,
)
from .component_lifecycle_terminal import terminal
from .component_manifest import ComponentManifest
from .component_resolver import verify_post_operation_inventory
from .component_transaction_state import clear_journal as _clear_journal
from .component_transaction_state import write_inventory
from .component_transition_journal import Journal
from .plugin_resolution import MarketplaceBinding


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]
InstalledLister = Callable[[Path, Runner], object]


def finish_committed(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
    hook_lister: HookLister | None,
    list_installed: InstalledLister,
    clear: Callable[[Path], None] = _clear_journal,
) -> dict[str, object]:
    stored = admit_pending_receipt(home, manifest, journal)
    if stored is not None:
        clear(home)
        return stored
    inventory = list_installed(executable, invoke)
    installed = verify_post_operation_inventory(
        manifest, inventory, journal.target, root
    )
    activation = (
        activation_for_inventory(
            manifest, inventory, root, executable, home, hook_lister=hook_lister
        )
        if journal.command in {"install", "update", "bootstrap"}
        else {}
    )
    activation_errors = tuple(dict.fromkeys(activation.values()))
    outcome = "pending-action" if activation_errors else "completed"
    receipt = journal.receipt(outcome, installed, activation_errors)
    if matching_receipt(home, manifest, receipt.encode()):
        clear(home)
        return receipt.encode()
    if (
        replay_receipt(
            home, manifest, journal.identifier, journal.command, journal.requested
        )
        is not None
    ):
        raise ValueError(
            f"operation receipt conflicts with committed transaction: {journal.identifier}"
        )
    write_inventory(home, installed)
    encoded = terminal(home, manifest, receipt)
    clear(home)
    return encoded
