"""Host staging, activation, verification, and recovery for monolith migration."""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path
from typing import Callable

from .component_inspection import doctor, status
from .component_lifecycle import run_operation
from .component_manifest import load_component_manifest
from .component_transaction_state import (
    InventorySnapshot,
    clear_journal as clear_component_journal,
    read_journal as read_component_journal,
    restore_inventory_snapshot,
)
from .github_pre_session import run_github_pre_session
from .monolith_classifier import classify_monolith
from .monolith_migration_state import MigrationJournal
from .plugin_resolution import official_install
from .pre_session import (
    _json,
    _run,
    reconcile_official_marketplace_root,
    run_pre_session,
)


Runner = Callable[[list[str]], subprocess.CompletedProcess[str] | None]
Discover = Callable[[Path, Path, Runner], tuple[Path, str]]


def stage_target(executable: Path, target: str, selection: tuple[str, ...]) -> None:
    """Prove a split target in an isolated Codex home before active replacement."""
    with tempfile.TemporaryDirectory(prefix="getcodexy-monolith-stage-") as directory:
        home = Path(directory) / "home"
        runner = lambda command: _run(command, home)
        run_pre_session(home, codex=executable, runner=runner, package_version=target)
        if "github" in selection:
            run_github_pre_session(
                home,
                codex=executable,
                runner=runner,
                package_version=target,
            )
        result = run_operation("install", selection, home, executable, runner)
        if result.get("outcome") != "completed":
            raise RuntimeError("staged split component activation did not complete")
        verify_split(home, executable, runner, selection)


def activate(home: Path, executable: Path, runner: Runner, plan: object) -> None:
    source = getattr(plan, "source_version")
    target = getattr(plan, "target_version")
    selection = tuple(getattr(plan, "selection"))
    if not isinstance(source, str) or not isinstance(target, str):
        raise RuntimeError("migration plan has invalid release identities")
    if load_component_manifest().version != target:
        raise RuntimeError("split component manifest does not match the target release")
    run_pre_session(home, codex=executable, runner=runner, package_version=target)
    if "github" in selection:
        run_github_pre_session(
            home,
            codex=executable,
            runner=runner,
            package_version=target,
        )
    result = run_operation(
        "install", selection, home, executable, runner, lock_held=True
    )
    if result.get("outcome") != "completed":
        raise RuntimeError("split component activation did not complete")
    verify_split(home, executable, runner, selection)


def verify_split(
    home: Path, executable: Path, runner: Runner, selection: tuple[str, ...]
) -> None:
    report = doctor(home, codex=executable, runner=runner)
    states = {
        entry.get("component"): entry.get("state")
        for entry in report["component_health"]
        if isinstance(entry, dict)
    }
    if any(states.get(component) != "healthy" for component in selection):
        raise RuntimeError("split component health did not pass")
    observed = status(home, codex=executable, runner=runner)
    if observed["errors"] or tuple(observed["installed_components"]) != selection:
        raise RuntimeError("split component inventory did not converge")


def rollback(
    home: Path,
    executable: Path,
    runner: Runner,
    journal: MigrationJournal,
    discover: Discover,
) -> None:
    remove_split_components(executable, runner, journal.selection)
    marketplace = reconcile_official_marketplace_root(
        executable, runner, journal.source_version, home
    )
    _restore_legacy_install(executable, runner, marketplace, journal.source_version)
    journal.snapshot.restore()
    if journal.snapshot != journal.snapshot.capture(home):
        raise RuntimeError("legacy configuration snapshot did not restore")
    root, version = discover(home, executable, runner)
    if (
        version != journal.source_version
        or classify_monolith(root).state != "supported-unmodified"
    ):
        raise RuntimeError("legacy monolith did not restore exactly")
    require_split_extensions_absent(executable, runner)
    _restore_component_transaction(home, journal.selection)


def _restore_legacy_install(
    executable: Path, runner: Runner, marketplace: Path, source_version: str
) -> None:
    """Restore the exact legacy plugin without applying split-only registration."""
    _json(
        runner([str(executable), "plugin", "add", "codexy@codexy", "--json"]),
        "legacy plugin add",
    )
    root, version = official_install(
        _json(
            runner([str(executable), "plugin", "list", "--json"]), "legacy plugin list"
        ),
        marketplace,
        source_version,
    )
    if (
        version != source_version
        or classify_monolith(root).state != "supported-unmodified"
    ):
        raise RuntimeError("legacy monolith install did not restore exactly")


def _restore_component_transaction(home: Path, selection: tuple[str, ...]) -> None:
    pending = read_component_journal(home)
    if pending is None:
        return
    if pending.target != selection:
        raise RuntimeError("nested lifecycle transaction does not match migration")
    restore_inventory_snapshot(home, pending.snapshot)
    if InventorySnapshot.capture(home) != pending.snapshot:
        raise RuntimeError("nested lifecycle inventory did not restore")
    clear_component_journal(home)


def require_split_extensions_absent(executable: Path, runner: Runner) -> None:
    payload = _json(
        runner([str(executable), "plugin", "list", "--json"]), "plugin list"
    )
    installed = payload.get("installed") if isinstance(payload, dict) else None
    extensions = {"codexy-github@codexy", "codexy-devtools@codexy"}
    if any(
        isinstance(item, dict)
        and item.get("enabled") is True
        and item.get("pluginId") in extensions
        for item in installed or ()
    ):
        raise RuntimeError("split extensions remained after legacy rollback")


def remove_split_components(
    executable: Path, runner: Runner, selection: tuple[str, ...]
) -> None:
    payload = _json(
        runner([str(executable), "plugin", "list", "--json"]), "plugin list"
    )
    installed = payload.get("installed") if isinstance(payload, dict) else None
    identifiers = {
        item.get("pluginId")
        for item in installed or ()
        if isinstance(item, dict) and isinstance(item.get("pluginId"), str)
    }
    for component in ("devtools", "github"):
        identifier = f"codexy-{component}@codexy"
        if component in selection and identifier in identifiers:
            _json(
                runner([str(executable), "plugin", "remove", identifier, "--json"]),
                "plugin remove",
            )
