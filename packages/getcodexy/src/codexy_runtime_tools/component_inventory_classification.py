"""Classify host plugin records against the component manifest."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from .component_manifest import DOMAIN_ERRORS, Component, ComponentManifest


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


def classify_installed_inventory(
    manifest: ComponentManifest, inventory: object
) -> ClassifiedInstalledInventory:
    """Classify all host records once through exact manifest identities."""
    if not isinstance(inventory, dict) or not isinstance(
        inventory.get("installed"), list
    ):
        raise ComponentResolutionError("invalid-installed-inventory")
    by_plugin = {component.plugin: component for component in manifest.components}
    records = []
    for entry in inventory["installed"]:
        if not isinstance(entry, dict):
            raise ComponentResolutionError("invalid-installed-inventory")
        plugin, plugin_id, marketplace = (
            entry.get("name"),
            entry.get("pluginId"),
            entry.get("marketplaceName"),
        )
        identified_plugin, identified_marketplace = plugin_id_parts(plugin_id)
        named = by_plugin.get(plugin) if isinstance(plugin, str) else None
        identified = by_plugin.get(identified_plugin)
        if named is not None or identified is not None:
            component = named or identified
            canonical = (
                named is component
                and identified is component
                and plugin == component.plugin
                and plugin_id == component.asset.plugin_id
                and marketplace == manifest.marketplace.name
            )
            records.append(
                ClassifiedInstalledRecord(
                    entry, component, InstalledIdentity.KNOWN, canonical
                )
            )
        elif (
            valid_identity_triple(
                plugin, identified_plugin, marketplace, identified_marketplace
            )
            and marketplace == manifest.marketplace.name
        ):
            records.append(
                ClassifiedInstalledRecord(entry, None, InstalledIdentity.UNKNOWN, False)
            )
        elif not valid_identity_triple(
            plugin, identified_plugin, marketplace, identified_marketplace
        ):
            records.append(
                ClassifiedInstalledRecord(
                    entry, None, InstalledIdentity.MALFORMED, False
                )
            )
        else:
            records.append(
                ClassifiedInstalledRecord(
                    entry, None, InstalledIdentity.IRRELEVANT, False
                )
            )
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


def plugin_id_parts(value: object) -> tuple[str | None, str | None]:
    if not isinstance(value, str) or value.count("@") != 1:
        return None, None
    plugin, marketplace = value.split("@")
    return plugin or None, marketplace or None


def valid_identity_triple(
    plugin: object,
    identifier_plugin: str | None,
    marketplace: object,
    identifier_marketplace: str | None,
) -> bool:
    return (
        isinstance(plugin, str)
        and bool(plugin)
        and isinstance(marketplace, str)
        and bool(marketplace)
        and identifier_plugin == plugin
        and identifier_marketplace == marketplace
    )
