"""Deterministic component planning from requests and the host plugin inventory."""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

from .component_manifest import DOMAIN_ERRORS, Component, ComponentManifest, valid_semver


SEMVER = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")


class ComponentResolutionError(ValueError):
    def __init__(self, code: str, components: tuple[str, ...] = ()) -> None:
        if code not in DOMAIN_ERRORS:
            raise ValueError(f"unknown getcodexy component domain error: {code}")
        super().__init__(code)
        self.code = code
        self.components = components


class InstalledIdentity(str, Enum):
    IRRELEVANT = "irrelevant"
    KNOWN = "known"
    UNKNOWN = "unknown"
    MALFORMED = "malformed"


@dataclass(frozen=True)
class ClassifiedInstalledRecord:
    entry: dict[str, object]
    component: Component | None
    identity: InstalledIdentity
    canonical: bool


@dataclass(frozen=True)
class ClassifiedInstalledInventory:
    records: tuple[ClassifiedInstalledRecord, ...]


def resolve_components(manifest: ComponentManifest, requested: tuple[str, ...] | list[str]) -> tuple[str, ...]:
    requested = tuple(requested)
    if unknown := tuple(component for component in requested if component not in manifest.component_ids):
        raise ComponentResolutionError("unknown-component", unknown)
    if len(set(requested)) != len(requested):
        raise ComponentResolutionError("conflicting-component-request")
    selected = set(requested or manifest.component_ids)
    while True:
        expanded = selected | {dependency for component in manifest.components if component.id in selected for dependency in component.dependencies}
        if expanded == selected:
            break
        selected = expanded
    resolved = tuple(component for component in manifest.component_ids if component in selected)
    if resolved not in manifest.compatible_combinations:
        raise ComponentResolutionError("incompatible-component-selection")
    return resolved


def reconcile_installed_inventory(manifest: ComponentManifest, inventory: object, marketplace_root: Path) -> tuple[str, ...]:
    return _reconcile_classified_inventory(manifest, classify_installed_inventory(manifest, inventory), marketplace_root)


def admit_installed_inventory(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None) -> tuple[str, ...]:
    """Complete host inventory admission before lifecycle mutation or recovery."""
    classified = classify_installed_inventory(manifest, inventory)
    if marketplace_root is None:
        preflight_unregistered_inventory(classified)
        return ()
    return _reconcile_classified_inventory(manifest, classified, marketplace_root)


def _reconcile_classified_inventory(manifest: ComponentManifest, classified: ClassifiedInstalledInventory, marketplace_root: Path) -> tuple[str, ...]:
    records = _component_records(manifest, classified, marketplace_root)
    versions = {record["version"] for record in records.values()}
    if len(versions) > 1:
        raise ComponentResolutionError("mixed-version-state")
    if versions and _version_tuple(next(iter(versions))) > _version_tuple(manifest.version):
        raise ComponentResolutionError("component-version-mismatch")
    selected = tuple(component for component in manifest.component_ids if component in records)
    if selected not in manifest.compatible_combinations:
        raise ComponentResolutionError("inconsistent-installed-state")
    return selected


def verify_post_operation_inventory(manifest: ComponentManifest, inventory: object, expected: tuple[str, ...], marketplace_root: Path) -> tuple[str, ...]:
    selected = reconcile_installed_inventory(manifest, inventory, marketplace_root)
    records = _component_records(manifest, classify_installed_inventory(manifest, inventory), marketplace_root)
    if selected != expected:
        raise ComponentResolutionError("installed-state-mismatch")
    if any(record["version"] != manifest.version for record in records.values()):
        raise ComponentResolutionError("component-version-mismatch")
    return selected


def classify_installed_inventory(manifest: ComponentManifest, inventory: object) -> ClassifiedInstalledInventory:
    """Classify all host records once through exact manifest identities."""
    if not isinstance(inventory, dict) or not isinstance(inventory.get("installed"), list):
        raise ComponentResolutionError("invalid-installed-inventory")
    by_plugin = {component.plugin: component for component in manifest.components}
    records = []
    for entry in inventory["installed"]:
        if not isinstance(entry, dict):
            raise ComponentResolutionError("invalid-installed-inventory")
        plugin, plugin_id, marketplace = entry.get("name"), entry.get("pluginId"), entry.get("marketplaceName")
        identifier_plugin, identifier_marketplace = _plugin_id_parts(plugin_id)
        named = by_plugin.get(plugin) if isinstance(plugin, str) else None
        identified = by_plugin.get(identifier_plugin)
        if named is not None or identified is not None:
            component = named or identified
            canonical = named is component and identified is component and plugin == component.plugin and plugin_id == component.asset.plugin_id and marketplace == manifest.marketplace.name
            records.append(ClassifiedInstalledRecord(entry, component, InstalledIdentity.KNOWN, canonical))
        elif marketplace == manifest.marketplace.name or identifier_marketplace == manifest.marketplace.name:
            records.append(ClassifiedInstalledRecord(entry, None, InstalledIdentity.UNKNOWN, False))
        elif not isinstance(plugin, str) or identifier_plugin is None or not isinstance(marketplace, str):
            records.append(ClassifiedInstalledRecord(entry, None, InstalledIdentity.MALFORMED, False))
        else:
            records.append(ClassifiedInstalledRecord(entry, None, InstalledIdentity.IRRELEVANT, False))
    return ClassifiedInstalledInventory(tuple(records))


def preflight_unregistered_inventory(inventory: ClassifiedInstalledInventory) -> None:
    """Reject manifest-relevant records while the marketplace is not registered."""
    for record in inventory.records:
        if record.identity is InstalledIdentity.MALFORMED:
            raise ComponentResolutionError("invalid-installed-inventory")
        if record.identity is InstalledIdentity.KNOWN:
            raise ComponentResolutionError("conflicting-installed-state")
        if record.identity is InstalledIdentity.UNKNOWN:
            raise ComponentResolutionError("unknown-installed-component")


def _plugin_id_parts(value: object) -> tuple[str | None, str | None]:
    if not isinstance(value, str) or value.count("@") != 1:
        return None, None
    plugin, marketplace = value.split("@")
    return plugin or None, marketplace or None


def _component_records(manifest: ComponentManifest, inventory: ClassifiedInstalledInventory, marketplace_root: Path) -> dict[str, dict[str, object]]:
    if not marketplace_root.is_absolute():
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
        if component is None or not record.canonical or not _valid_record(record.entry, component, manifest, marketplace_root) or component.id in records:
            raise ComponentResolutionError("conflicting-installed-state")
        records[component.id] = record.entry
    return records


def _valid_record(entry: dict[str, object], component: Component, manifest: ComponentManifest, marketplace_root: Path) -> bool:
    asset = component.asset
    source = entry.get("source")
    if not isinstance(source, dict) or source.get("source") != "local" or not isinstance(source.get("path"), str) or not Path(source["path"]).is_absolute():
        return False
    if Path(source["path"]) != marketplace_root / asset.package_root:
        return False
    return entry.get("pluginId") == asset.plugin_id and entry.get("marketplaceName") == manifest.marketplace.name and entry.get("marketplaceSource") == {"sourceType": "git", "source": manifest.marketplace.source} and entry.get("installed") is True and entry.get("enabled") is True and valid_semver(entry.get("version"))


def _version_tuple(version: str) -> tuple[int, int, int]:
    return tuple(int(part) for part in version.split("."))  # type: ignore[return-value]
