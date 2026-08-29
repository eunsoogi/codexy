"""Validate classified installed records against their expected source roots."""

from __future__ import annotations

from pathlib import Path

from .component_inventory_classification import (
    ClassifiedInstalledInventory,
    ComponentResolutionError,
    InstalledIdentity,
)
from .component_manifest import Component, ComponentManifest, valid_semver
from .plugin_resolution import MarketplaceBinding, marketplace_path, marketplace_source


def component_records(
    manifest: ComponentManifest,
    inventory: ClassifiedInstalledInventory,
    marketplace_root: MarketplaceBinding,
) -> dict[str, dict[str, object]]:
    root = marketplace_path(marketplace_root)
    if not root.is_absolute():
        raise ComponentResolutionError("invalid-installed-inventory")
    records = {}
    for record in inventory.records:
        if record.identity is InstalledIdentity.MALFORMED:
            raise ComponentResolutionError("invalid-installed-inventory")
        if record.identity is InstalledIdentity.IRRELEVANT:
            continue
        if record.identity is InstalledIdentity.UNKNOWN:
            raise ComponentResolutionError("unknown-installed-component")
        component = record.component
        if (
            component is None
            or not record.canonical
            or not valid_record(
                record.entry,
                component,
                manifest,
                root,
                marketplace_source(marketplace_root),
            )
            or component.id in records
        ):
            raise ComponentResolutionError("conflicting-installed-state")
        records[component.id] = record.entry
    return records


def valid_record(
    entry: dict[str, object],
    component: Component,
    manifest: ComponentManifest,
    marketplace_root: Path,
    expected_source: dict[str, str] | None = None,
) -> bool:
    if not valid_observed_record(entry, component, manifest, expected_source):
        return False
    source = entry.get("source")
    expected = marketplace_root / component.asset.package_root
    return (
        isinstance(source, dict)
        and isinstance(source.get("path"), str)
        and source["path"] == str(expected)
        and Path(source["path"]) == expected
    )


def valid_observed_record(
    entry: dict[str, object],
    component: Component,
    manifest: ComponentManifest,
    expected_source: dict[str, str] | None = None,
) -> bool:
    """Validate the identity a failed marketplace probe can still establish."""
    source = entry.get("source")
    if (
        not isinstance(source, dict)
        or source.get("source") != "local"
        or not isinstance(source.get("path"), str)
    ):
        return False
    path, expected = Path(source["path"]), Path(component.asset.package_root)
    return (
        path.is_absolute()
        and path.parts[-len(expected.parts) :] == expected.parts
        and entry.get("pluginId") == component.asset.plugin_id
        and entry.get("marketplaceName") == manifest.marketplace.name
        and entry.get("marketplaceSource")
        == (
            expected_source
            or {"sourceType": "git", "source": manifest.marketplace.source}
        )
        and entry.get("installed") is True
        and entry.get("enabled") is True
        and valid_semver(entry.get("version"))
    )
