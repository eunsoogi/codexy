"""Read-only doctor entries for managed role files whose component is absent."""

from __future__ import annotations

from pathlib import Path

from .component_manifest import ComponentManifest
from .component_registration_health import REGISTRATION_SCOPE, REGISTRATION_SOURCE
from .component_registration_projection import MANAGED_ROOTS, managed_role_files


def include_orphaned_registrations(
    entries: list[dict[str, object]],
    manifest: ComponentManifest,
    actual: tuple[str, ...],
    home: Path | None,
) -> list[dict[str, object]]:
    """Inspect role roots even when neither recorded nor live inventory names them."""
    if home is None:
        return entries
    by_component = {entry["component"]: entry for entry in entries}
    for component in manifest.component_ids:
        if component not in MANAGED_ROOTS or component in actual:
            continue
        try:
            names = managed_role_files(home, component)
            inspection_error = False
        except (OSError, ValueError):
            names = ()
            inspection_error = True
        if not names and not inspection_error:
            continue
        repair = (
            "remove only this absent component's Codexy-managed role files, then rerun getcodexy doctor"
            if names
            else "inspect the Codexy registration directory, then rerun getcodexy doctor"
        )
        reason = (
            "orphaned-managed-registration"
            if names
            else "registration-inspection-unavailable"
        )
        registration = {
            "component": component,
            "source": REGISTRATION_SOURCE,
            "scope": REGISTRATION_SCOPE,
            "observed": True,
            "state": "stale" if names else "unmanaged-conflict",
            "roles": [],
            "managed_extra": list(names),
            "unmanaged_extra": [],
            "expected_count": 0,
            "exact_count": 0,
            "cause": "managed role files remain for an absent component"
            if names
            else "registration directory could not be inspected safely",
            "recovery": repair,
        }
        entry = by_component.get(component)
        if entry is None:
            new_entry: dict[str, object] = {
                "component": component,
                "state": registration["state"],
                "installed": False,
                "configured": False,
                "started": False,
                "callable": False,
                "healthy": False,
                "first_failure_stage": "registration",
                "reason_code": reason,
                "safe_fallback": repair,
                "restart_required": False,
                "repair": repair,
                "observed": {"registration": registration},
            }
            entries.append(new_entry)
            by_component[component] = new_entry
            continue
        entry.update(
            state=registration["state"],
            healthy=False,
            first_failure_stage="registration",
            reason_code=reason,
            safe_fallback=repair,
            restart_required=False,
            repair=repair,
        )
        observed = entry.get("observed")
        if not isinstance(observed, dict):
            observed = {}
            entry["observed"] = observed
        observed["registration"] = registration
    return entries
