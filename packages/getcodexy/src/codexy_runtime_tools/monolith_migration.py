"""Safe public admission seam for monolithic Codexy migrations."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Callable

from .component_inspection import doctor, status
from .component_lifecycle import PreAdmissionError
from .component_manifest import load_component_manifest
from .component_resolver import ComponentResolutionError, resolve_components
from .monolith_migration_host import activate as _activate
from .monolith_migration_host import rollback as _host_rollback
from .monolith_migration_host import stage_target as _stage_target
from .monolith_migration_plan import MigrationPlan, plan_migration
from .monolith_migration_state import (
    MigrationJournal,
    clear_journal,
    read_journal,
    write_journal,
)
from .plugin_resolution import official_marketplace
from .pre_session import _json
from .updater import _absolute, _validate_real_path
from .component_transaction_state import transaction_lock
from .version_lock import default_package_version


Runner = Callable[[list[str]], subprocess.CompletedProcess[str] | None]


def migrate(
    home: Path,
    executable: Path,
    runner: Runner,
    requested: tuple[str, ...] = (),
) -> dict[str, object]:
    try:
        absolute_home = _absolute(home)
        _validate_real_path(absolute_home, require_exists=False)
    except (OSError, RuntimeError, ValueError):
        return _rejected(default_package_version(), "ambiguous-monolith")
    try:
        with transaction_lock(absolute_home):
            return _migrate(absolute_home, executable, runner, requested)
    except PreAdmissionError:
        return _rejected(default_package_version(), "migration-in-progress")


def _migrate(
    home: Path,
    executable: Path,
    runner: Runner,
    requested: tuple[str, ...],
) -> dict[str, object]:
    try:
        pending = read_journal(home)
    except (OSError, ValueError):
        return _rejected(default_package_version(), "corrupt-migration-journal")
    if pending is not None:
        rolling = pending.with_phase("rolling-back")
        write_journal(home, rolling)
        _rollback(home, executable, runner, rolling)
        clear_journal(home)
        return _receipt("rolled-back", rolling, "interrupted-migration")
    target = default_package_version()
    if receipt := _already_migrated(home, executable, runner, target, requested):
        return receipt
    try:
        root, source_version = _discover(home, executable, runner)
    except (OSError, RuntimeError, ValueError):
        return _rejected(target, "ambiguous-monolith")
    plan = plan_migration(root, target, requested)
    if plan.source_version != source_version:
        return _rejected(target, "ambiguous-monolith")
    if plan.outcome != "ready":
        return _receipt("rejected", plan, plan.error)
    _stage_target(executable, target, plan.selection)
    journal = MigrationJournal.capture(
        home, str(plan.source_version), plan.target_version, plan.selection
    )
    write_journal(home, journal)
    try:
        journal = journal.with_phase("activating")
        write_journal(home, journal)
        _activate(home, executable, runner, plan)
    except BaseException:
        rolling = journal.with_phase("rolling-back")
        write_journal(home, rolling)
        _rollback(home, executable, runner, rolling)
        clear_journal(home)
        return _receipt("rolled-back", plan, "operation-failed")
    clear_journal(home)
    return _receipt("completed", plan, None)


def _discover(home: Path, executable: Path, runner: Runner) -> tuple[Path, str]:
    marketplace = official_marketplace(
        _json(
            runner([str(executable), "plugin", "marketplace", "list", "--json"]),
            "marketplace list",
        )
    )
    payload = _json(
        runner([str(executable), "plugin", "list", "--json"]), "plugin list"
    )
    installed = payload.get("installed") if isinstance(payload, dict) else None
    matches = [
        item
        for item in installed or ()
        if isinstance(item, dict)
        and item.get("pluginId") == "codexy@codexy"
        and item.get("enabled") is True
    ]
    if len(matches) != 1:
        raise RuntimeError("expected exactly one enabled legacy Codexy core plugin")
    source = matches[0].get("source")
    root = source.get("path") if isinstance(source, dict) else None
    version = matches[0].get("version")
    if (
        not isinstance(root, str)
        or Path(root) != marketplace / "plugins" / "codexy"
        or not isinstance(version, str)
    ):
        raise RuntimeError("legacy Codexy plugin identity is ambiguous")
    return Path(root), version


def _already_migrated(
    home: Path,
    executable: Path,
    runner: Runner,
    target: str,
    requested: tuple[str, ...],
) -> dict[str, object] | None:
    try:
        manifest = load_component_manifest()
        if manifest.version != target:
            return None
        selection = resolve_components(manifest, requested)
        observed = status(home, codex=executable, runner=runner)
        report = doctor(home, codex=executable, runner=runner)
    except (ComponentResolutionError, OSError, RuntimeError, ValueError):
        return None
    states = {
        entry.get("component"): entry.get("state")
        for entry in report["component_health"]
        if isinstance(entry, dict)
    }
    if (
        observed["errors"]
        or tuple(observed["installed_components"]) != selection
        or any(states.get(component) != "healthy" for component in selection)
    ):
        return None
    plan = MigrationPlan("ready", target, target, selection, None, "already migrated")
    return _receipt("completed", plan, None)


def _rollback(
    home: Path, executable: Path, runner: Runner, journal: MigrationJournal
) -> None:
    _host_rollback(home, executable, runner, journal, _discover)


def _receipt(outcome: str, plan: object, error: str | None) -> dict[str, object]:
    selection = list(getattr(plan, "selection"))
    return {
        "schema": "getcodexy.monolith-migration-receipt.v1",
        "command": "migrate",
        "outcome": outcome,
        "source_version": getattr(plan, "source_version"),
        "target_version": getattr(plan, "target_version"),
        "selection_after": selection if outcome == "completed" else [],
        "errors": [] if error is None else [{"code": error}],
        "recovery": getattr(plan, "recovery", "restore the recorded legacy release"),
    }


def _rejected(target: str, error: str) -> dict[str, object]:
    return _receipt(
        "rejected",
        MigrationPlan(
            "rejected",
            None,
            target,
            (),
            error,
            "preserve this installation and recover it manually",
        ),
        error,
    )
