"""Deterministic component planning from requests and the host plugin inventory."""

from __future__ import annotations

from pathlib import Path

from .component_inventory_classification import (
    ClassifiedInstalledInventory,
    ComponentResolutionError,
    classify_installed_inventory,
    preflight_unregistered_inventory,
)
from .component_inventory_records import component_records, valid_observed_record
from .component_manifest import ComponentManifest, valid_semver


def resolve_components(
    manifest: ComponentManifest, requested: tuple[str, ...] | list[str]
) -> tuple[str, ...]:
    requested = tuple(requested)
    if unknown := tuple(
        component for component in requested if component not in manifest.component_ids
    ):
        raise ComponentResolutionError("unknown-component", unknown)
    if len(set(requested)) != len(requested):
        raise ComponentResolutionError("conflicting-component-request")
    selected = set(requested or manifest.component_ids)
    while True:
        expanded = selected | {
            dependency
            for component in manifest.components
            if component.id in selected
            for dependency in component.dependencies
        }
        if expanded == selected:
            break
        selected = expanded
    resolved = tuple(
        component for component in manifest.component_ids if component in selected
    )
    if resolved not in manifest.compatible_combinations:
        raise ComponentResolutionError("incompatible-component-selection")
    return resolved


def canonical_components(
    manifest: ComponentManifest, components: set[str]
) -> tuple[str, ...]:
    return tuple(
        component for component in manifest.component_ids if component in components
    )


def reconcile_installed_inventory(
    manifest: ComponentManifest, inventory: object, marketplace_root: Path
) -> tuple[str, ...]:
    return _reconcile_classified_inventory(
        manifest, classify_installed_inventory(manifest, inventory), marketplace_root
    )


def admit_installed_inventory(
    manifest: ComponentManifest, inventory: object, marketplace_root: Path | None
) -> tuple[str, ...]:
    """Complete host inventory admission before lifecycle mutation or recovery."""
    classified = classify_installed_inventory(manifest, inventory)
    if marketplace_root is None:
        preflight_unregistered_inventory(classified)
        return ()
    return _reconcile_classified_inventory(manifest, classified, marketplace_root)


def admit_operation_inventory(
    manifest: ComponentManifest,
    inventory: object,
    marketplace_root: Path | None,
    command: str,
) -> tuple[str, ...]:
    """Require current retained components unless this operation will upgrade them."""
    if command not in {"install", "update", "remove", "bootstrap"}:
        raise ValueError(f"unsupported component operation: {command}")
    selected = admit_installed_inventory(manifest, inventory, marketplace_root)
    if command in {"update", "bootstrap"} or marketplace_root is None:
        return selected
    records = component_records(
        manifest, classify_installed_inventory(manifest, inventory), marketplace_root
    )
    if any(record["version"] != manifest.version for record in records.values()):
        raise ComponentResolutionError("component-version-mismatch")
    return selected


def admit_recovery_inventory(
    manifest: ComponentManifest,
    inventory: object,
    marketplace_root: Path | None,
    expected: tuple[str, ...],
) -> tuple[str, ...]:
    """Admit a pending transaction's host state without rejecting its own mixed-version update."""
    if expected not in manifest.compatible_combinations:
        raise ComponentResolutionError("inconsistent-installed-state")
    classified = classify_installed_inventory(manifest, inventory)
    if marketplace_root is None:
        selected = admit_installed_inventory(manifest, inventory, None)
    else:
        records = component_records(manifest, classified, marketplace_root)
        versions = {record["version"] for record in records.values()}
        if any(
            _version_tuple(version) > _version_tuple(manifest.version)
            for version in versions
        ):
            raise ComponentResolutionError("component-version-mismatch")
        if len(versions - {manifest.version}) > 1:
            raise ComponentResolutionError("mixed-version-state")
        selected = canonical_components(manifest, set(records))
    if selected != expected:
        raise ComponentResolutionError("inconsistent-installed-state")
    return selected


def admit_bootstrap_recovery_inventory(
    manifest: ComponentManifest,
    inventory: object,
    marketplace_root: Path | None,
    before: tuple[str, ...],
    target: tuple[str, ...],
) -> tuple[str, ...]:
    """Admit only a canonical add-only bootstrap state authorized by its journal."""
    selected = admit_operation_inventory(
        manifest, inventory, marketplace_root, "bootstrap"
    )
    if not set(before).issubset(selected) or not set(selected).issubset(target):
        raise ComponentResolutionError("inconsistent-installed-state")
    return selected


def _reconcile_classified_inventory(
    manifest: ComponentManifest,
    classified: ClassifiedInstalledInventory,
    marketplace_root: Path,
) -> tuple[str, ...]:
    records = component_records(manifest, classified, marketplace_root)
    versions = {record["version"] for record in records.values()}
    if len(versions) > 1:
        raise ComponentResolutionError("mixed-version-state")
    if versions and _version_tuple(next(iter(versions))) > _version_tuple(
        manifest.version
    ):
        raise ComponentResolutionError("component-version-mismatch")
    selected = tuple(
        component for component in manifest.component_ids if component in records
    )
    if selected not in manifest.compatible_combinations:
        raise ComponentResolutionError("inconsistent-installed-state")
    return selected


def verify_post_operation_inventory(
    manifest: ComponentManifest,
    inventory: object,
    expected: tuple[str, ...],
    marketplace_root: Path,
) -> tuple[str, ...]:
    selected = reconcile_installed_inventory(manifest, inventory, marketplace_root)
    records = component_records(
        manifest, classify_installed_inventory(manifest, inventory), marketplace_root
    )
    if selected != expected:
        raise ComponentResolutionError("installed-state-mismatch")
    if any(record["version"] != manifest.version for record in records.values()):
        raise ComponentResolutionError("component-version-mismatch")
    return selected


def compare_versions(left: str, right: str) -> int:
    """Compare two manifest-valid semantic versions without reimplementing parsing."""
    if not valid_semver(left) or not valid_semver(right):
        raise ComponentResolutionError("component-version-mismatch")
    return (_version_tuple(left) > _version_tuple(right)) - (
        _version_tuple(left) < _version_tuple(right)
    )


def _version_tuple(version: str) -> tuple[int, int, int]:
    return tuple(int(part) for part in version.split("."))  # type: ignore[return-value]
