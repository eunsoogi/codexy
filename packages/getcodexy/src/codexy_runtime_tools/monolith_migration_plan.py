"""Pure admission for a legacy-to-split migration."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .component_manifest import load_component_manifest
from .component_resolver import ComponentResolutionError, resolve_components
from .monolith_classifier import classify_monolith


@dataclass(frozen=True)
class MigrationPlan:
    outcome: str
    source_version: str | None
    target_version: str
    selection: tuple[str, ...]
    error: str | None
    recovery: str


def plan_migration(
    root: Path, target_version: str, requested: tuple[str, ...]
) -> MigrationPlan:
    classification = classify_monolith(root)
    if classification.state != "supported-unmodified":
        return MigrationPlan(
            "rejected",
            classification.version,
            target_version,
            (),
            f"{classification.state}-monolith",
            classification.recovery,
        )
    if classification.version == target_version:
        return MigrationPlan(
            "rejected",
            classification.version,
            target_version,
            (),
            "target-release-unavailable",
            "a distinct split release is required",
        )
    try:
        selection = resolve_components(load_component_manifest(), requested)
    except ComponentResolutionError as error:
        return MigrationPlan(
            "rejected",
            classification.version,
            target_version,
            (),
            error.code,
            str(error),
        )
    return MigrationPlan(
        "ready", classification.version, target_version, selection, None, ""
    )
