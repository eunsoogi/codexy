"""Non-mutating validation helpers for component lifecycle operations."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Callable

from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError, resolve_components
from .component_transaction_state import read_inventory
from .plugin_resolution import (
    MarketplaceBinding,
    marketplace_identity,
    marketplace_path,
    named_marketplace,
    validate_local_marketplace,
)
from .pre_session import _json


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


def validate_request(
    command: str, requested: tuple[str, ...], manifest: ComponentManifest
) -> None:
    if command == "bootstrap" and requested:
        raise ComponentResolutionError("components-not-accepted")
    if command == "remove" and not requested:
        raise ComponentResolutionError("missing-removal-target")
    if command == "install" or requested:
        resolve_components(manifest, requested)


def existing_marketplace(
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest | None = None,
) -> MarketplaceBinding | None:
    payload = _json(
        invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
        "plugin marketplace list",
    )
    binding = marketplace_identity(payload) if named_marketplace(payload) else None
    if binding is not None and manifest is not None:
        validate_local_marketplace(
            binding,
            manifest.version,
            tuple(component.plugin for component in manifest.components),
        )
    return binding


def existing_marketplace_root(executable: Path, invoke: Runner) -> Path | None:
    binding = existing_marketplace(executable, invoke)
    return None if binding is None else marketplace_path(binding)


def recorded_selection(
    home: Path, manifest: ComponentManifest
) -> tuple[str, ...] | None:
    selected = read_inventory(home)
    if selected is None:
        return None
    canonical = tuple(
        component for component in manifest.component_ids if component in selected
    )
    if selected != canonical or selected not in manifest.compatible_combinations:
        raise ValueError("installed component inventory is inconsistent")
    return selected
