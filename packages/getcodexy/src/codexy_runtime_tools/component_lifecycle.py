"""Transactional public lifecycle operations backed by the component resolver."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Callable

from .component_lifecycle_admission import admit_pending_receipt, admitted_selection, matching_receipt, replay_receipt
from .component_manifest import ComponentManifest, load_component_manifest
from .component_lifecycle_preflight import existing_marketplace_root, recorded_selection, validate_request
from .component_resolver import ComponentResolutionError, canonical_components, reconcile_installed_inventory, resolve_components, verify_post_operation_inventory
from .component_transaction_identity import operation_id
from .component_transaction_receipts import operation_receipt, write_receipt
from .component_transaction_state import InventorySnapshot, Journal, clear_journal, decode_inventory, inventory_path, read_journal, transaction_lock, write_inventory, write_journal
from .github_pre_session import trusted_codex
from .pre_session import _find_codex, _json, _run, official_marketplace_root
from .updater import _absolute, _validate_real_path


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


def run_operation(command: str, requested: tuple[str, ...], codex_home: str | os.PathLike[str], codex: Path | None = None, runner: Runner | None = None, *, operation_id: str | None = None) -> dict[str, object]:
    """Run a serialized operation, recovering any preceding interrupted operation first."""
    if command not in {"install", "update", "remove"}:
        raise ValueError(f"unsupported component operation: {command}")
    home = _absolute(codex_home)
    _validate_real_path(home, require_exists=False)
    executable, invoke = trusted_codex(codex or _find_codex()), runner or (lambda args: _run(args, home))
    manifest, identifier = load_component_manifest(), _operation_id(operation_id)
    with transaction_lock(home):
        pending = read_journal(home)
        if pending is not None:
            _validate_journal(pending, manifest)
            pending_receipt = admit_pending_receipt(home, manifest, pending)
        else:
            pending_receipt = None
        replay = replay_receipt(home, manifest, identifier, command, requested)
        if replay is not None and pending is None:
            return replay
        try:
            validate_request(command, requested, manifest)
            recorded = recorded_selection(home, manifest)
        except ComponentResolutionError as error:
            return _terminal(home, operation_receipt(identifier, command, requested, (), (), (), "rejected", error.code))
        except (OSError, ValueError, RuntimeError):
            return _terminal(home, operation_receipt(identifier, command, requested, (), (), (), "rejected", "inconsistent-installed-state"))
        try:
            root = existing_marketplace_root(executable, invoke)
            inventory = _list(executable, invoke)
            before = admitted_selection(manifest, inventory, root)
        except ComponentResolutionError as error:
            return _terminal(home, operation_receipt(identifier, command, requested, (), (), (), "rejected", error.code))
        except (OSError, ValueError, RuntimeError):
            return _terminal(home, operation_receipt(identifier, command, requested, (), (), (), "rejected", "inconsistent-installed-state"))
        if pending is not None:
            if pending.phase == "rolling-back" and pending_receipt is not None:
                if before != pending.before or recorded != pending.before:
                    raise ValueError("pending transaction receipt does not match restored state")
                clear_journal(home)
                if replay is not None:
                    return replay
                pending = None
            if pending is not None:
                _recover_if_needed(home, executable, invoke, manifest, root or official_marketplace_root(executable, invoke))
            root = existing_marketplace_root(executable, invoke)
            inventory = _list(executable, invoke)
            before = admitted_selection(manifest, inventory, root)
            recorded = recorded_selection(home, manifest)
            if replay is not None:
                return replay
        try:
            if recorded is not None and recorded != before:
                raise ComponentResolutionError("inconsistent-installed-state")
            resolved, target, adds, removes = _plan(command, requested, before, recorded, manifest)
        except ComponentResolutionError as error:
            return _terminal(home, operation_receipt(identifier, command, requested, (), before, before, "rejected", error.code))
        except (OSError, ValueError, RuntimeError):
            return _terminal(home, operation_receipt(identifier, command, requested, (), before, before, "rejected", "inconsistent-installed-state"))

        journal = Journal(identifier, command, requested, resolved, before, target, InventorySnapshot.capture(home), "started")
        write_journal(home, journal)
        try:
            root = root or official_marketplace_root(executable, invoke)
            installed = _apply_forward(executable, invoke, manifest, root, journal, adds, removes)
        except BaseException as error:
            if root is None:
                raise RuntimeError("component operation failed; durable recovery is required") from error
            _rollback_or_raise(home, executable, invoke, manifest, root, journal, error)
            receipt = _terminal(home, operation_receipt(identifier, command, requested, resolved, before, before, "rolled-back", "operation-failed"))
            clear_journal(home)
            return receipt
        return _write_completed(home, executable, invoke, manifest, root, journal, installed)


def _operation_id(value: str | None) -> str:
    identifier = operation_id(value)
    if value is not None and value != identifier:
        raise ValueError("operation ID must be a safe op- identifier")
    return identifier


def _recover_if_needed(home: Path, executable: Path, invoke: Runner, manifest: ComponentManifest, root: Path) -> None:
    journal = read_journal(home)
    if journal is None:
        return
    _validate_journal(journal, manifest)
    if journal.phase == "committed":
        _finish_committed(home, executable, invoke, manifest, root, journal)
        return
    if journal.command == "update" and journal.phase == "started":
        try:
            installed = _apply_forward(executable, invoke, manifest, root, journal, journal.resolved, ())
        except BaseException as error:
            _rollback_or_raise(home, executable, invoke, manifest, root, journal, error)
            _terminal(home, operation_receipt(journal.identifier, journal.command, journal.requested, journal.resolved, journal.before, journal.before, "rolled-back", "operation-failed"))
            clear_journal(home)
            return
        _write_completed(home, executable, invoke, manifest, root, journal, installed)
        return
    if journal.phase == "started":
        try:
            installed = verify_post_operation_inventory(manifest, _list(executable, invoke), journal.target, root)
        except ComponentResolutionError:
            installed = None
        if installed is not None:
            _write_completed(home, executable, invoke, manifest, root, journal, installed)
            return
    _rollback_or_raise(home, executable, invoke, manifest, root, journal, RuntimeError("interrupted component operation"))
    _terminal(home, operation_receipt(journal.identifier, journal.command, journal.requested, journal.resolved, journal.before, journal.before, "rolled-back", "operation-failed"))
    clear_journal(home)


def _rollback_or_raise(home: Path, executable: Path, invoke: Runner, manifest: ComponentManifest, root: Path, journal: Journal, cause: BaseException) -> None:
    try:
        write_journal(home, journal.with_phase("rolling-back"))
        _restore_selection(executable, invoke, manifest, root, journal.before)
        restored = _selection(manifest, _list(executable, invoke), root)
        if restored != journal.before:
            raise RuntimeError("restored selection did not match the operation snapshot")
        journal.snapshot.restore(home)
    except BaseException as rollback_error:
        raise RuntimeError("component operation failed; durable recovery is required") from rollback_error


def _write_completed(home: Path, executable: Path, invoke: Runner, manifest: ComponentManifest, root: Path, journal: Journal, installed: tuple[str, ...]) -> dict[str, object]:
    write_inventory(home, installed)
    write_journal(home, journal.with_phase("committed"))
    return _finish_committed(home, executable, invoke, manifest, root, journal)


def _finish_committed(home: Path, executable: Path, invoke: Runner, manifest: ComponentManifest, root: Path, journal: Journal) -> dict[str, object]:
    installed = verify_post_operation_inventory(manifest, _list(executable, invoke), journal.target, root)
    receipt = operation_receipt(journal.identifier, journal.command, journal.requested, journal.resolved, journal.before, installed, "completed")
    if matching_receipt(home, manifest, receipt):
        clear_journal(home)
        return receipt
    if replay_receipt(home, manifest, journal.identifier, journal.command, journal.requested) is not None:
        raise ValueError(f"operation receipt conflicts with committed transaction: {journal.identifier}")
    write_inventory(home, installed)
    receipt = _terminal(home, receipt)
    clear_journal(home)
    return receipt


def _plan(command: str, requested: tuple[str, ...], before: tuple[str, ...], recorded: tuple[str, ...] | None, manifest: ComponentManifest) -> tuple[tuple[str, ...], tuple[str, ...], tuple[str, ...], tuple[str, ...]]:
    if command == "install":
        resolved = resolve_components(manifest, requested)
        target = canonical_components(manifest, set(before) | set(resolved))
        return resolved, target, canonical_components(manifest, set(target) - set(before)), ()
    if command == "update":
        if recorded is None:
            raise ComponentResolutionError("no-recorded-selection")
        resolved = before if not requested else resolve_components(manifest, requested)
        if not set(resolved).issubset(before):
            raise ComponentResolutionError("incompatible-component-selection")
        return resolved, before, resolved, ()
    if not requested:
        raise ComponentResolutionError("missing-removal-target")
    resolve_components(manifest, requested)
    resolved = canonical_components(manifest, set(requested))
    target = canonical_components(manifest, set(before) - set(resolved))
    if target not in manifest.compatible_combinations:
        raise ComponentResolutionError("dependency-protected-removal")
    return resolved, target, (), tuple(reversed(resolved))


def _restore_selection(executable: Path, invoke: Runner, manifest: ComponentManifest, root: Path, before: tuple[str, ...]) -> None:
    """Restore the selection without replacing a coherent prior-version component."""
    current = _selection(manifest, _list(executable, invoke), root)
    for component in before:
        if component not in current:
            _mutate(executable, invoke, "add", manifest, component)
    for component in reversed(manifest.component_ids):
        if component in current and component not in before:
            _mutate(executable, invoke, "remove", manifest, component)


def _apply_forward(executable: Path, invoke: Runner, manifest: ComponentManifest, root: Path, journal: Journal, adds: tuple[str, ...], removes: tuple[str, ...]) -> tuple[str, ...]:
    if journal.command == "update":
        _json(invoke([str(executable), "plugin", "marketplace", "upgrade", "codexy", "--json"]), "plugin marketplace upgrade")
        root = official_marketplace_root(executable, invoke)
    for component in adds:
        _mutate(executable, invoke, "add", manifest, component)
    for component in removes:
        _mutate(executable, invoke, "remove", manifest, component)
    return verify_post_operation_inventory(manifest, _list(executable, invoke), journal.target, root)


def _mutate(executable: Path, invoke: Runner, action: str, manifest: ComponentManifest, component_id: str) -> None:
    _json(invoke([str(executable), "plugin", action, manifest.component(component_id).asset.plugin_id, "--json"]), f"plugin {action}")


def _list(executable: Path, invoke: Runner) -> object:
    return _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list")


def _selection(manifest: ComponentManifest, payload: object, root: Path) -> tuple[str, ...]:
    return reconcile_installed_inventory(manifest, payload, root)


def _validate_journal(journal: Journal, manifest: ComponentManifest) -> None:
    if journal.command not in {"install", "update", "remove"} or any(
        value != canonical_components(manifest, set(value)) for value in (journal.before, journal.target, journal.resolved)
    ) or journal.before not in manifest.compatible_combinations or journal.target not in manifest.compatible_combinations:
        raise ValueError("component transaction journal is inconsistent")
    snapshot = journal.snapshot.contents
    if snapshot is None:
        if journal.before:
            raise ValueError("component transaction journal is missing its inventory snapshot")
    else:
        if decode_inventory(snapshot) != journal.before:
            raise ValueError("component transaction journal does not match its inventory snapshot")
    recorded = journal.before
    try:
        resolved, target, _, _ = _plan(journal.command, journal.requested, journal.before, recorded, manifest)
    except ComponentResolutionError as error:
        raise ValueError("component transaction journal has an invalid request") from error
    if (resolved, target) != (journal.resolved, journal.target):
        raise ValueError("component transaction journal does not match its plan")


def _terminal(home: Path, receipt: dict[str, object]) -> dict[str, object]:
    write_receipt(home, receipt)
    return receipt
