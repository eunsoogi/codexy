"""Authoritative package data for Codexy component selection."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from importlib.resources import files
from itertools import combinations
from typing import Any



SCHEMA = "getcodexy.component-manifest.v1"
OFFICIAL = "https://github.com/eunsoogi/codexy.git"
MARKETPLACE = "codexy"
COMPONENT_IDS = ("core", "github", "devtools")
SEMVER = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
MAX_SEMVER_COMPONENT = 2_147_483_647
DOMAIN_ERRORS = frozenset(
    {
        "component-version-mismatch",
        "components-not-accepted",
        "conflicting-component-request",
        "conflicting-installed-state",
        "dependency-protected-removal",
        "incompatible-component-selection",
        "inconsistent-installed-state",
        "installed-state-mismatch",
        "invalid-installed-inventory",
        "missing-removal-target",
        "mixed-version-state",
        "no-recorded-selection",
        "operation-failed",
        "unknown-component",
        "unknown-installed-component",
    }
)


@dataclass(frozen=True)
class Marketplace:
    name: str
    source: str


@dataclass(frozen=True)
class Asset:
    plugin_id: str
    package_root: str
    required_paths: tuple[str, ...]


@dataclass(frozen=True)
class Component:
    id: str
    plugin: str
    version: str
    dependencies: tuple[str, ...]
    asset: Asset


@dataclass(frozen=True)
class ComponentManifest:
    marketplace: Marketplace
    version: str
    components: tuple[Component, ...]
    compatible_combinations: tuple[tuple[str, ...], ...]
    domain_errors: frozenset[str]

    @property
    def component_ids(self) -> tuple[str, ...]:
        return tuple(component.id for component in self.components)

    def component(self, component_id: str) -> Component:
        return next(component for component in self.components if component.id == component_id)


def load_component_manifest() -> ComponentManifest:
    return parse_component_manifest(files("codexy_runtime_tools").joinpath("component-manifest.json").read_text())


def parse_component_manifest(text: str) -> ComponentManifest:
    return _parse_manifest(json.loads(text, object_pairs_hook=_unique_object))


def valid_semver(value: object) -> bool:
    return isinstance(value, str) and SEMVER.fullmatch(value) is not None and all(
        len(component) < len(str(MAX_SEMVER_COMPONENT)) or len(component) == len(str(MAX_SEMVER_COMPONENT)) and component <= str(MAX_SEMVER_COMPONENT)
        for component in value.split(".")
    )


def _parse_manifest(data: object) -> ComponentManifest:
    fields = {"schema", "marketplace", "domainErrors", "components", "compatibleCombinations"}
    if not isinstance(data, dict) or set(data) != fields or data["schema"] != SCHEMA:
        raise ValueError("invalid getcodexy component manifest")
    marketplace = _marketplace(data["marketplace"])
    domain_errors = _domain_errors(data["domainErrors"])
    if not isinstance(data["components"], list) or not isinstance(data["compatibleCombinations"], list):
        raise ValueError("component manifest is missing components or compatibility")
    components = tuple(_component(item, marketplace) for item in data["components"])
    ids, versions = tuple(component.id for component in components), {component.version for component in components}
    if ids != COMPONENT_IDS or len(ids) != len(set(ids)) or len(versions) != 1:
        raise ValueError("component manifest IDs and versions must be lockstep")
    if len({component.plugin for component in components}) != len(components) or len({component.asset.plugin_id for component in components}) != len(components) or len({component.asset.package_root for component in components}) != len(components):
        raise ValueError("component manifest assets must uniquely identify components")
    if any(dependency not in ids or dependency == component.id or component.dependencies.count(dependency) > 1 for component in components for dependency in component.dependencies):
        raise ValueError("component manifest has invalid dependencies")
    if _has_cycle(components):
        raise ValueError("component manifest dependencies must not cycle")
    version = next(iter(versions))
    compatible = tuple(_combination(item, ids, version) for item in data["compatibleCombinations"])
    expected = _compatible_combinations(components)
    if set(compatible) != expected or len(compatible) != len(expected):
        raise ValueError("component manifest compatible combinations are incomplete")
    return ComponentManifest(marketplace, version, components, compatible, domain_errors)


def _marketplace(value: object) -> Marketplace:
    if not isinstance(value, dict) or set(value) != {"name", "source"} or not all(isinstance(value[key], str) and value[key] for key in value) or value["name"] != MARKETPLACE or value["source"] != OFFICIAL:
        raise ValueError("component manifest marketplace has an invalid shape")
    return Marketplace(value["name"], value["source"])


def _domain_errors(value: object) -> frozenset[str]:
    if not isinstance(value, dict) or set(value) != DOMAIN_ERRORS or any(
        not isinstance(description, str) or not description
        for description in value.values()
    ):
        raise ValueError("component manifest domain errors are not closed")
    return frozenset(value)


def _component(value: object, marketplace: Marketplace) -> Component:
    required = {"id", "plugin", "version", "dependencies", "asset"}
    if not isinstance(value, dict) or set(value) != required:
        raise ValueError("component manifest component has an invalid shape")
    if any(not isinstance(value[key], str) or not value[key] for key in ("id", "plugin", "version")) or not valid_semver(value["version"]):
        raise ValueError("component manifest component has invalid text")
    return Component(value["id"], value["plugin"], value["version"], _strings(value["dependencies"], "dependencies"), _asset(value["asset"], value["plugin"], marketplace))


def _asset(value: object, plugin: str, marketplace: Marketplace) -> Asset:
    if not isinstance(value, dict) or set(value) != {"pluginId", "packageRoot", "requiredPaths"}:
        raise ValueError("component manifest asset has an invalid shape")
    root, paths = value.get("packageRoot"), _strings(value.get("requiredPaths"), "requiredPaths", nonempty=True)
    if value.get("pluginId") != f"{plugin}@{marketplace.name}" or root != f"plugins/{plugin}" or len(paths) != len(set(paths)) or any(path.startswith("/") or ".." in path.split("/") for path in paths):
        raise ValueError("component manifest asset is not canonical")
    return Asset(value["pluginId"], root, paths)


def _combination(value: object, ids: tuple[str, ...], version: str) -> tuple[str, ...]:
    if not isinstance(value, dict) or set(value) != {"components", "version"}:
        raise ValueError("component manifest compatibility has an invalid shape")
    components = _strings(value.get("components"), "components")
    if value.get("version") != version or components != tuple(item for item in ids if item in components):
        raise ValueError("component manifest compatibility is not canonical")
    return components


def _compatible_combinations(components: tuple[Component, ...]) -> set[tuple[str, ...]]:
    ids, dependencies = tuple(component.id for component in components), {component.id: set(component.dependencies) for component in components}
    return {subset for size in range(len(ids) + 1) for subset in combinations(ids, size) if all(dependencies[item].issubset(subset) for item in subset)}


def _has_cycle(components: tuple[Component, ...]) -> bool:
    dependencies = {component.id: component.dependencies for component in components}
    visiting, visited = set(), set()
    def visit(component: str) -> bool:
        if component in visiting:
            return True
        if component in visited:
            return False
        visiting.add(component)
        cyclic = any(visit(dependency) for dependency in dependencies[component])
        visiting.remove(component)
        visited.add(component)
        return cyclic
    return any(visit(component) for component in dependencies)


def _strings(value: Any, field: str, *, nonempty: bool = False) -> tuple[str, ...]:
    if not isinstance(value, list) or (nonempty and not value) or any(not isinstance(item, str) or not item for item in value):
        raise ValueError(f"component manifest {field} must be strings")
    return tuple(value)


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"component manifest has duplicate key: {key}")
        result[key] = value
    return result
