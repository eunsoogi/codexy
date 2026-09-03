"""Read-only activation checks for packaged Codexy hooks."""

from __future__ import annotations

from pathlib import Path
from typing import Callable

from .component_hook_activation_expected import ExpectedHook, expected_hooks
from .component_hook_activation_host import (
    HOOK_STATE_UNAVAILABLE,
    HookStateError,
    list_hooks,
)
from .component_hook_activation_validation import classify_activation
from .component_inventory_classification import classify_installed_inventory
from .component_inventory_records import component_records
from .component_manifest import ComponentManifest
from .component_resolver import canonical_components
from .plugin_resolution import MarketplaceBinding


HOOK_COMPONENTS = ("core", "github")
ACTIVATION_REPAIRS = {
    "required-hook-trust-missing": (
        "approve the pending Codexy hooks in Codex, then rerun getcodexy doctor",
        True,
    ),
    "required-hook-disabled": (
        "enable the required Codexy hooks in Codex, then rerun getcodexy doctor",
        True,
    ),
    "required-hook-trust-stale": (
        "refresh trust for the installed Codexy hooks in Codex, then rerun getcodexy doctor",
        True,
    ),
    HOOK_STATE_UNAVAILABLE: (
        "inspect Codex hook state, then rerun getcodexy doctor",
        True,
    ),
}
ACTIVATION_ERRORS = frozenset(ACTIVATION_REPAIRS)
ACTIVATION_STATES = {
    "required-hook-trust-missing": "pending-trust",
    "required-hook-disabled": "pending-trust",
    "required-hook-trust-stale": "stale",
}
HookLister = Callable[[Path, Path], object]


def activation_for_inventory(
    manifest: ComponentManifest,
    inventory: object,
    root: MarketplaceBinding,
    executable: Path,
    codex_home: Path,
    *,
    hook_lister: HookLister | None = None,
) -> dict[str, str]:
    """Return one activation failure code per installed hook component."""
    try:
        classified = classify_installed_inventory(manifest, inventory)
        records = component_records(manifest, classified, root)
        actual = canonical_components(manifest, set(records))
    except (OSError, ValueError):
        return {component: HOOK_STATE_UNAVAILABLE for component in HOOK_COMPONENTS}
    components = tuple(
        component for component in HOOK_COMPONENTS if component in actual
    )
    if not components:
        return {}
    try:
        rows = (hook_lister or list_hooks)(executable, codex_home)
        expected = expected_hooks(manifest, components, records)
        return classify_activation(expected, rows, codex_home=codex_home)
    except HookStateError:
        return {component: HOOK_STATE_UNAVAILABLE for component in components}
    except (OSError, TypeError, ValueError):
        return {component: HOOK_STATE_UNAVAILABLE for component in components}


__all__ = [
    "ACTIVATION_ERRORS",
    "ACTIVATION_REPAIRS",
    "ACTIVATION_STATES",
    "ExpectedHook",
    "HOOK_COMPONENTS",
    "HOOK_STATE_UNAVAILABLE",
    "HookLister",
    "HookStateError",
    "activation_for_inventory",
    "classify_activation",
    "expected_hooks",
    "list_hooks",
]
