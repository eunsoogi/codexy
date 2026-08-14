"""Resolver-owned admission for plugin-list observations without a marketplace root."""

from __future__ import annotations

from dataclasses import dataclass

from .component_manifest import ComponentManifest
from .component_resolver import (
    ClassifiedInstalledInventory,
    ComponentResolutionError,
    InstalledIdentity,
    canonical_components,
    classify_installed_inventory,
    compare_versions,
    valid_observed_record,
)


@dataclass(frozen=True)
class ObservedInstalledInventory:
    """Canonical managed selection plus the exact plugin records used to admit it."""

    selection: tuple[str, ...]
    records: dict[str, dict[str, object]]
    error: str | None


def observe_installed_inventory(manifest: ComponentManifest, inventory: object) -> ObservedInstalledInventory:
    """Read-only fallback admission when marketplace discovery failed after plugin-list."""
    classified: ClassifiedInstalledInventory | None = None
    records: dict[str, dict[str, object]] = {}
    try:
        classified = classify_installed_inventory(manifest, inventory)
        records = _recognized_records(manifest, classified)
        return ObservedInstalledInventory(_admit(manifest, classified, records), records, None)
    except ComponentResolutionError as error:
        return ObservedInstalledInventory(_recognized_selection(manifest, classified), records, error.code)


def _recognized_records(manifest: ComponentManifest, classified: ClassifiedInstalledInventory) -> dict[str, dict[str, object]]:
    records = {}
    for record in classified.records:
        if record.component is not None:
            records.setdefault(record.component.id, record.entry)
    return records


def _recognized_selection(manifest: ComponentManifest, classified: ClassifiedInstalledInventory | None) -> tuple[str, ...]:
    if classified is None:
        return ()
    return canonical_components(manifest, {record.component.id for record in classified.records if record.component is not None})


def _admit(manifest: ComponentManifest, classified: ClassifiedInstalledInventory, records: dict[str, dict[str, object]]) -> tuple[str, ...]:
    for record in classified.records:
        if record.identity is InstalledIdentity.MALFORMED:
            raise ComponentResolutionError("invalid-installed-inventory")
        if record.identity is InstalledIdentity.IRRELEVANT:
            continue
        if record.identity is InstalledIdentity.UNKNOWN:
            raise ComponentResolutionError("unknown-installed-component")
        component = record.component
        if component is None or not record.canonical or not valid_observed_record(record.entry, component, manifest) or sum(item.component is component for item in classified.records) != 1:
            raise ComponentResolutionError("conflicting-installed-state")
    versions = {str(record["version"]) for record in records.values()}
    if len(versions) > 1:
        raise ComponentResolutionError("mixed-version-state")
    if versions and compare_versions(next(iter(versions)), manifest.version) > 0:
        raise ComponentResolutionError("component-version-mismatch")
    selection = canonical_components(manifest, set(records))
    if selection not in manifest.compatible_combinations:
        raise ComponentResolutionError("inconsistent-installed-state")
    return selection
