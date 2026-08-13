"""Read-only status and doctor reports from admitted host observations."""

from __future__ import annotations

import os
import subprocess
from enum import Enum
from pathlib import Path
from typing import Callable

from .component_diagnostic_health import health
from .component_manifest import ComponentManifest, load_component_manifest
from .component_observed_inventory import observe_installed_inventory
from .component_resolver import ComponentResolutionError, admit_inspected_inventory
from .component_source_admission import DiagnosticTree
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


def status(codex_home: str | os.PathLike[str], *, codex: Path | None = None, runner: Runner | None = None) -> dict[str, object]:
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


def doctor(codex_home: str | os.PathLike[str], *, codex: Path | None = None, runner: Runner | None = None) -> dict[str, object]:
    report = _inspect(codex_home, codex, runner)
    host_error = report["host_error"]
    readiness = {"state": "error", "missing_requirements": [host_error]} if host_error else {"state": "ready", "missing_requirements": []}
    return {
        "schema": DOCTOR_SCHEMA,
        "command": "doctor",
        "outcome": "completed",
        "inventory": report["inventory"],
        "inventory_consistency": report["consistency"],
        "host_readiness": readiness,
        "component_health": health(
            report["manifest"],
            report["actual"],
            report["recorded"],
            report["records"],
            report["trees"],
            report["admission_error"],
            host_error == ProbeStage.MARKETPLACE_LIST.value,
        ),
        "source_of_truth": "installed-component-inventory",
        "errors": report["errors"],
    }


def _inspect(codex_home: str | os.PathLike[str], codex: Path | None, runner: Runner | None) -> dict[str, object]:
    home, manifest = _absolute(codex_home), load_component_manifest()
    _validate_real_path(home, require_exists=False)
    recorded, inventory, inventory_error = _recorded(home)
    executable, invoke, probe = _host(home, codex, runner)
    actual: tuple[str, ...] = ()
    records: dict[str, dict[str, object]] = {}
    trees: dict[str, DiagnosticTree] = {}
    admission_error: str | None = None
    host_error = probe.value if probe else None
    if probe is None:
        try:
            installed = _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list")
        except (OSError, RuntimeError, ValueError):
            host_error = ProbeStage.PLUGIN_LIST.value
        else:
            try:
                root = _marketplace_root(executable, invoke)
            except (OSError, RuntimeError, ValueError):
                actual, records, admission_error = _observed(manifest, installed)
                host_error = ProbeStage.MARKETPLACE_LIST.value
            else:
                actual, records, trees, admission_error = _actual(manifest, installed, root)
    errors = [{"code": code} for code in (host_error, admission_error, inventory_error) if code]
    inconsistent = bool(host_error or admission_error or inventory_error or recorded is not None and recorded != actual)
    return {
        "manifest": manifest,
        "actual": actual,
        "recorded": recorded,
        "records": records,
        "trees": trees,
        "admission_error": admission_error,
        "host_error": host_error,
        "inventory": inventory,
        "consistency": (
            "inconsistent" if inconsistent else "not-recorded" if recorded is None else "consistent"
        ),
        "errors": errors or ([{"code": "inconsistent-installed-state"}] if inconsistent else []),
    }


def _host(home: Path, codex: Path | None, runner: Runner | None) -> tuple[Path | None, Runner | None, ProbeStage | None]:
    try:
        return trusted_codex(codex or _find_codex()), runner or (lambda command: _run(command, home)), None
    except (OSError, RuntimeError, ValueError):
        return None, None, ProbeStage.EXECUTABLE


def _recorded(home: Path) -> tuple[tuple[str, ...] | None, dict[str, object], str | None]:
    try:
        recorded = read_inventory(home)
    except (OSError, ValueError):
        return None, {"state": "invalid"}, "inconsistent-installed-state"
    return (None, {"state": "absent"}, None) if recorded is None else (recorded, {"state": "present", "components": list(recorded)}, None)


def _marketplace_root(executable: Path, invoke: Runner) -> Path | None:
    payload = _json(invoke([str(executable), "plugin", "marketplace", "list", "--json"]), "plugin marketplace list")
    return official_marketplace(payload) if named_marketplace(payload) else None


def _actual(manifest: ComponentManifest, installed: object, root: Path | None) -> tuple[tuple[str, ...], dict[str, dict[str, object]], dict[str, DiagnosticTree], str | None]:
    actual, records, _ = _observed(manifest, installed)
    try:
        admitted = admit_inspected_inventory(manifest, installed, root)
        if root is None:
            return admitted.selection, {}, {}, None
        if actual != admitted.selection:
            raise ComponentResolutionError("inconsistent-installed-state")
        return actual, records, admitted.trees, None
    except ComponentResolutionError as error:
        return actual, records, {}, error.code
    except (OSError, ValueError):
        return actual, records, {}, "invalid-installed-inventory"


def _observed(manifest: ComponentManifest, installed: object) -> tuple[tuple[str, ...], dict[str, dict[str, object]], str | None]:
    observed = observe_installed_inventory(manifest, installed)
    return observed.selection, observed.records, observed.error
