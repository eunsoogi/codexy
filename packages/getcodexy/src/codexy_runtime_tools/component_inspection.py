"""Read-only component status and diagnostic reports."""

from __future__ import annotations

import os
import subprocess
from enum import Enum
from pathlib import Path
from typing import Callable

from .component_manifest import ComponentManifest, load_component_manifest
from .component_observed_inventory import observe_installed_inventory
from .component_health import health as _health
from .component_resolver import (
    ComponentResolutionError,
    admit_installed_inventory,
    canonical_components,
    classify_installed_inventory,
)
from .component_transaction_state import read_inventory
from .github_pre_session import trusted_codex
from .plugin_resolution import named_marketplace, official_marketplace
from .pre_session import _find_codex, _json, _run
from .updater import _absolute, _validate_real_path


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]
STATUS_SCHEMA = "getcodexy.status.v1"
DOCTOR_SCHEMA = "getcodexy.doctor.v1"


class ProbeStage(str, Enum):
    EXECUTABLE = "codex-executable"
    PLUGIN_LIST = "codex-plugin-list"
    MARKETPLACE_LIST = "codex-marketplace-list"


def status(
    codex_home: str | os.PathLike[str],
    *,
    codex: Path | None = None,
    runner: Runner | None = None,
) -> dict[str, object]:
    """Report actual installed components without changing the Codex home."""
    report = _inspect(codex_home, codex, runner)
    return {
        "schema": STATUS_SCHEMA,
        "command": "status",
        "outcome": "completed",
        "inventory": report["inventory"],
        "inventory_consistency": report["consistency"],
        "selected_components": list(report["recorded"] or ()),
        "installed_components": list(report["actual"]),
        "source_of_truth": "installed-component-inventory",
        "errors": report["errors"],
    }


def doctor(
    codex_home: str | os.PathLike[str],
    *,
    codex: Path | None = None,
    runner: Runner | None = None,
) -> dict[str, object]:
    """Inspect canonical managed files and return actionable repairs."""
    report = _inspect(codex_home, codex, runner)
    host_error = report["host_error"]
    readiness = (
        {"state": "error", "missing_requirements": [host_error]}
        if host_error
        else {"state": "ready", "missing_requirements": []}
    )
    return {
        "schema": DOCTOR_SCHEMA,
        "command": "doctor",
        "outcome": "completed",
        "inventory": report["inventory"],
        "inventory_consistency": report["consistency"],
        "host_readiness": readiness,
        "component_health": _health(
            report["manifest"],
            report["actual"],
            report["recorded"],
            report["records"],
            report["admission_error"],
            bool(host_error),
        ),
        "source_of_truth": "installed-component-inventory",
        "errors": report["errors"],
    }


def _inspect(
    codex_home: str | os.PathLike[str], codex: Path | None, runner: Runner | None
) -> dict[str, object]:
    home, manifest = _absolute(codex_home), load_component_manifest()
    _validate_real_path(home, require_exists=False)
    recorded, inventory, inventory_error = _recorded(home)
    actual: tuple[str, ...] = ()
    records: dict[str, dict[str, object]] = {}
    admission_error: str | None = None
    executable, invoke, probe = _host(home, codex, runner)
    host_error = probe.value if probe else None
    if probe is None:
        try:
            installed = _json(
                invoke([str(executable), "plugin", "list", "--json"]), "plugin list"
            )
        except (OSError, RuntimeError, ValueError):
            host_error = ProbeStage.PLUGIN_LIST.value
        else:
            try:
                root = _marketplace_root(executable, invoke)
            except (OSError, RuntimeError, ValueError):
                observed = observe_installed_inventory(manifest, installed)
                actual, records, admission_error = (
                    observed.selection,
                    observed.records,
                    observed.error,
                )
                host_error = ProbeStage.MARKETPLACE_LIST.value
            else:
                actual, records, admission_error = _actual(manifest, installed, root)
    errors = ([{"code": "invalid-installed-inventory"}] if host_error else []) + [
        {"code": code} for code in (admission_error, inventory_error) if code
    ]
    inconsistent = bool(
        host_error
        or admission_error
        or inventory_error
        or recorded is not None
        and recorded != actual
    )
    return {
        "manifest": manifest,
        "actual": actual,
        "recorded": recorded,
        "records": records,
        "admission_error": admission_error,
        "host_error": host_error,
        "inventory": inventory,
        "consistency": "inconsistent"
        if inconsistent
        else "not-recorded"
        if recorded is None
        else "consistent",
        "errors": errors
        or ([{"code": "inconsistent-installed-state"}] if inconsistent else []),
    }


def _host(
    home: Path, codex: Path | None, runner: Runner | None
) -> tuple[Path | None, Runner | None, ProbeStage | None]:
    try:
        return (
            trusted_codex(codex or _find_codex()),
            runner or (lambda command: _run(command, home)),
            None,
        )
    except (OSError, RuntimeError, ValueError):
        return None, None, ProbeStage.EXECUTABLE


def _recorded(
    home: Path,
) -> tuple[tuple[str, ...] | None, dict[str, object], str | None]:
    try:
        recorded = read_inventory(home)
    except (OSError, ValueError):
        return None, {"state": "invalid"}, "inconsistent-installed-state"
    return (
        (None, {"state": "absent"}, None)
        if recorded is None
        else (recorded, {"state": "present", "components": list(recorded)}, None)
    )


def _marketplace_root(executable: Path, invoke: Runner) -> Path | None:
    payload = _json(
        invoke([str(executable), "plugin", "marketplace", "list", "--json"]),
        "plugin marketplace list",
    )
    return official_marketplace(payload) if named_marketplace(payload) else None


def _actual(
    manifest: ComponentManifest, installed: object, root: Path | None
) -> tuple[tuple[str, ...], dict[str, dict[str, object]], str | None]:
    actual: tuple[str, ...] = ()
    records: dict[str, dict[str, object]] = {}
    try:
        classified = classify_installed_inventory(manifest, installed)
        records = {
            record.component.id: record.entry
            for record in classified.records
            if record.component is not None
        }
        actual = canonical_components(manifest, set(records))
        admitted = admit_installed_inventory(manifest, installed, root)
        if root is None:
            return admitted, {}, None
        if actual != admitted:
            raise ComponentResolutionError("inconsistent-installed-state")
        return actual, records, None
    except ComponentResolutionError as error:
        return actual, records, error.code
    except (OSError, ValueError):
        return actual, records, "invalid-installed-inventory"
