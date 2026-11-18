"""Strict, transaction-only admission and local registration replacement."""

from __future__ import annotations

from pathlib import Path
from .component_inventory_classification import (
    classify_installed_inventory,
    InstalledIdentity,
)
from .component_inventory_records import valid_record
from .component_lifecycle_preflight import existing_marketplace, Runner
from .component_manifest import ComponentManifest
from .component_transition_journal import Journal
from .local_upgrade_state import LocalUpgrade
from .plugin_resolution import MarketplaceIdentity
from .pre_session import _json


def admit_partial(
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    journal: Journal,
    state: LocalUpgrade,
) -> tuple[str, ...]:
    """Only an exact durable transaction admits records spanning its two roots."""
    root = existing_marketplace(executable, invoke)
    if root not in (None, state.old, state.target):
        raise ValueError("local marketplace identity changed outside the transaction")
    inventory = _json(
        invoke([str(executable), "plugin", "list", "--json"]), "plugin list"
    )
    seen = set()
    for record in classify_installed_inventory(manifest, inventory).records:
        if record.identity is InstalledIdentity.IRRELEVANT:
            continue
        component = record.component
        if component is None or not record.canonical or component.id in seen:
            raise ValueError("local upgrade inventory has conflicting identities")
        if component.id not in set(journal.before) | set(journal.target):
            raise ValueError("local upgrade inventory has an unselected component")
        matches = [
            binding
            for binding, version in (
                (state.old, state.old_version),
                (state.target, state.target_version),
            )
            if record.entry.get("version") == version
            and valid_record(
                record.entry, component, manifest, binding.root, binding.host_source
            )
        ]
        if len(matches) != 1:
            raise ValueError(
                "local upgrade inventory is not bound to either verified archive"
            )
        seen.add(component.id)
    return tuple(c for c in manifest.component_ids if c in seen)


def switch(
    executable: Path,
    invoke: Runner,
    target: MarketplaceIdentity,
    allowed: tuple[MarketplaceIdentity, ...],
) -> None:
    current = existing_marketplace(executable, invoke)
    if current == target:
        return
    if current not in (None, *allowed):
        raise ValueError("refusing to replace an unrelated marketplace")
    if current is not None:
        _json(
            invoke(
                [str(executable), "plugin", "marketplace", "remove", "codexy", "--json"]
            ),
            "marketplace remove",
        )
    _json(
        invoke(
            [
                str(executable),
                "plugin",
                "marketplace",
                "add",
                str(target.root),
                "--json",
            ]
        ),
        "marketplace add",
    )
    if existing_marketplace(executable, invoke) != target:
        raise ValueError("local marketplace registration readback mismatch")
