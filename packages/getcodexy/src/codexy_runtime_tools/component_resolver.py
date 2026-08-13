"""Deterministic component planning from requests and the host plugin inventory."""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

from .component_manifest import DOMAIN_ERRORS, Component, ComponentManifest, valid_semver
from .component_source_admission import DiagnosticTree, diagnostic_paths, trusted_component_root

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


@dataclass(frozen=True)
class InspectedInstalledInventory:
    selection: tuple[str, ...]
    trees: dict[str, DiagnosticTree]

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


def canonical_components(manifest: ComponentManifest, components: set[str]) -> tuple[str, ...]:
    return tuple(component for component in manifest.component_ids if component in components)


def reconcile_installed_inventory(manifest: ComponentManifest, inventory: object, marketplace_root: Path) -> tuple[str, ...]:
    return _reconcile_classified_inventory(manifest, classify_installed_inventory(manifest, inventory), marketplace_root)


def admit_inspected_inventory(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None) -> InspectedInstalledInventory:
    """Admit an inventory before read-only diagnostics dereference component roots."""
    selected = admit_installed_inventory(manifest, inventory, marketplace_root)
    if marketplace_root is None:
        trees = {}
    else:
        components = tuple(manifest.component(component) for component in selected)
        if any(not trusted_component_root(marketplace_root, component) for component in components):
            raise ComponentResolutionError("conflicting-installed-state")
        trees = {component.id: DiagnosticTree(marketplace_root / component.asset.package_root) for component in components}
    if any(
        not trees[component].admits(diagnostic_paths(manifest.component(component)))
        for component in selected
    ):
        raise ComponentResolutionError("conflicting-installed-state")
    return InspectedInstalledInventory(selected, trees)


def admit_installed_inventory(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None) -> tuple[str, ...]:
    """Complete host inventory admission before lifecycle mutation or recovery."""
    classified = classify_installed_inventory(manifest, inventory)
    if marketplace_root is None:
        preflight_unregistered_inventory(classified)
        return ()
    return _reconcile_classified_inventory(manifest, classified, marketplace_root)


def admit_operation_inventory(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None, command: str) -> tuple[str, ...]:
    """Require current retained components unless this operation will upgrade them."""
    if command not in {"install", "update", "remove", "bootstrap"}:
        raise ValueError(f"unsupported component operation: {command}")
    selected = admit_installed_inventory(manifest, inventory, marketplace_root)
    if command in {"update", "bootstrap"} or marketplace_root is None:
        return selected
    records = _component_records(manifest, classify_installed_inventory(manifest, inventory), marketplace_root)
    if any(record["version"] != manifest.version for record in records.values()):
        raise ComponentResolutionError("component-version-mismatch")
    return selected


def admit_recovery_inventory(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None, expected: tuple[str, ...]) -> tuple[str, ...]:
    """Admit a pending transaction's host state without rejecting its own mixed-version update."""
    if expected not in manifest.compatible_combinations:
        raise ComponentResolutionError("inconsistent-installed-state")
    classified = classify_installed_inventory(manifest, inventory)
    if marketplace_root is None:
        selected = admit_installed_inventory(manifest, inventory, None)
    else:
        records = _component_records(manifest, classified, marketplace_root)
        versions = {record["version"] for record in records.values()}
        if any(_version_tuple(version) > _version_tuple(manifest.version) for version in versions):
            raise ComponentResolutionError("component-version-mismatch")
        if len(versions - {manifest.version}) > 1:
            raise ComponentResolutionError("mixed-version-state")
        selected = canonical_components(manifest, set(records))
    if selected != expected:
        raise ComponentResolutionError("inconsistent-installed-state")
    return selected


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


def compare_versions(left: str, right: str) -> int:
    """Compare two manifest-valid semantic versions without reimplementing parsing."""
    if not valid_semver(left) or not valid_semver(right):
        raise ComponentResolutionError("component-version-mismatch")
    return (_version_tuple(left) > _version_tuple(right)) - (_version_tuple(left) < _version_tuple(right))


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
        elif _valid_identity_triple(plugin, identifier_plugin, marketplace, identifier_marketplace) and marketplace == manifest.marketplace.name:
            records.append(ClassifiedInstalledRecord(entry, None, InstalledIdentity.UNKNOWN, False))
        elif not _valid_identity_triple(plugin, identifier_plugin, marketplace, identifier_marketplace):
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


def _valid_identity_triple(plugin: object, identifier_plugin: str | None, marketplace: object, identifier_marketplace: str | None) -> bool:
    return isinstance(plugin, str) and bool(plugin) and isinstance(marketplace, str) and bool(marketplace) and identifier_plugin == plugin and identifier_marketplace == marketplace


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
    if not valid_observed_record(entry, component, manifest):
        return False
    source = entry.get("source")
    expected = marketplace_root / component.asset.package_root
    return isinstance(source, dict) and isinstance(source.get("path"), str) and source["path"] == str(expected) and Path(source["path"]) == expected


def valid_observed_record(entry: dict[str, object], component: Component, manifest: ComponentManifest) -> bool:
    """Validate the identity a failed marketplace probe can still establish."""
    source = entry.get("source")
    if not isinstance(source, dict) or source.get("source") != "local" or not isinstance(source.get("path"), str):
        return False
    path, expected = Path(source["path"]), Path(component.asset.package_root)
    return path.is_absolute() and path.parts[-len(expected.parts):] == expected.parts and entry.get("pluginId") == component.asset.plugin_id and entry.get("marketplaceName") == manifest.marketplace.name and entry.get("marketplaceSource") == {"sourceType": "git", "source": manifest.marketplace.source} and entry.get("installed") is True and entry.get("enabled") is True and valid_semver(entry.get("version"))


def _version_tuple(version: str) -> tuple[int, int, int]:
    return tuple(int(part) for part in version.split("."))  # type: ignore[return-value]
