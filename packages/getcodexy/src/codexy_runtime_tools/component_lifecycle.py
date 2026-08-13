"""Transactional public lifecycle operations backed by the component resolver."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Callable

from .component_lifecycle_admission import admit_pending_receipt, admitted_recovery_selection, admitted_selection, matching_receipt, replay_receipt
from .component_manifest import ComponentManifest, load_component_manifest
from .component_lifecycle_preflight import existing_marketplace_root, recorded_selection, validate_request
from .component_resolver import ComponentResolutionError, reconcile_installed_inventory, verify_post_operation_inventory
from .component_transaction_identity import operation_id
from .component_transaction_receipts import write_receipt
from .component_transaction_state import InventorySnapshot, Journal, clear_journal, decode_inventory, inventory_path, read_journal, transaction_lock, write_inventory, write_journal
from .component_transition_model import OperationReceipt, Rejection, RejectionStage, StateFailure, plan_transition
from .github_pre_session import trusted_codex
from .pre_session import _find_codex, _json, _run, official_marketplace_root
from .updater import _absolute, _validate_real_path


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


def run_operation(command: str, requested: tuple[str, ...], codex_home: str | os.PathLike[str], codex: Path | None = None, runner: Runner | None = None, *, operation_id: str | None = None) -> dict[str, object]:
    """Run a serialized operation, recovering any preceding interrupted operation first."""
    if command not in {"install", "update", "remove", "bootstrap"}:
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
            return _reject(home, manifest, identifier, command, requested, (), RejectionStage.REQUEST, error)
        except (OSError, ValueError, RuntimeError):
            return _reject(home, manifest, identifier, command, requested, (), RejectionStage.PRESTATE, StateFailure.INCONSISTENT_INSTALLED_STATE)
        try:
            root = existing_marketplace_root(executable, invoke)
            inventory = _list(executable, invoke)
            before = admitted_recovery_selection(manifest, inventory, root, pending.target) if pending is not None and (pending.command, pending.phase) == ("update", "started") else admitted_selection(manifest, inventory, root, command)
        except ComponentResolutionError as error:
            return _reject(home, manifest, identifier, command, requested, (), RejectionStage.HOST, error)
        except (OSError, ValueError, RuntimeError):
            return _reject(home, manifest, identifier, command, requested, (), RejectionStage.HOST, StateFailure.INCONSISTENT_INSTALLED_STATE)
        if pending is not None:
            if pending.phase == "rolling-back" and pending_receipt is not None:
                if before != pending.before or (pending.snapshot.contents is None) != (recorded is None) or recorded not in {None, pending.before}:
                    raise ValueError("pending transaction receipt does not match restored state")
                clear_journal(home)
                if replay is not None:
                    return replay
                pending = None
            if pending is not None:
                _recover_if_needed(home, executable, invoke, manifest, root or official_marketplace_root(executable, invoke))
            root = existing_marketplace_root(executable, invoke)
            inventory = _list(executable, invoke)
            before = admitted_selection(manifest, inventory, root, command)
            recorded = recorded_selection(home, manifest)
            if replay is not None:
                return replay
        if recorded is not None and recorded != before and command != "bootstrap":
            return _reject(home, manifest, identifier, command, requested, before, RejectionStage.PRESTATE, StateFailure.INCONSISTENT_INSTALLED_STATE)
        try:
            plan = plan_transition(manifest, command, requested, before, recorded)
        except ComponentResolutionError as error:
            return _reject(home, manifest, identifier, command, requested, before, RejectionStage.PLAN, error)
        except (OSError, ValueError, RuntimeError):
            return _reject(home, manifest, identifier, command, requested, before, RejectionStage.PRESTATE, StateFailure.INCONSISTENT_INSTALLED_STATE)

        journal = plan.journal(identifier, InventorySnapshot.capture(home))
        write_journal(home, journal)
        try:
            root = root or official_marketplace_root(executable, invoke)
            installed = _apply_forward(executable, invoke, manifest, root, journal, plan.adds, plan.removes)
        except BaseException as error:
            if root is None:
                raise RuntimeError("component operation failed; durable recovery is required") from error
            _rollback_or_raise(home, executable, invoke, manifest, root, journal, error)
            receipt = _terminal(home, manifest, journal.receipt("rolled-back", journal.before))
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
            _terminal(home, manifest, journal.receipt("rolled-back", journal.before))
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
    _terminal(home, manifest, journal.receipt("rolled-back", journal.before))
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
    receipt = journal.receipt("completed", installed)
    if matching_receipt(home, manifest, receipt.encode()):
        clear_journal(home)
        return receipt.encode()
    if replay_receipt(home, manifest, journal.identifier, journal.command, journal.requested) is not None:
        raise ValueError(f"operation receipt conflicts with committed transaction: {journal.identifier}")
    write_inventory(home, installed)
    receipt = _terminal(home, manifest, receipt)
    clear_journal(home)
    return receipt


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
    journal.validate(manifest, decode_inventory)


def _reject(home: Path, manifest: ComponentManifest, identifier: str, command: str, requested: tuple[str, ...], before: tuple[str, ...], stage: RejectionStage, failure: ComponentResolutionError | StateFailure) -> dict[str, object]:
    rejection = Rejection.from_failure(stage, failure)
    rejection.validate(manifest, command, requested, before, plan_transition)
    return _terminal(home, manifest, OperationReceipt.rejected(identifier, command, requested, before, rejection))  # type: ignore[arg-type]


def _terminal(home: Path, manifest: ComponentManifest, receipt: OperationReceipt) -> dict[str, object]:
    receipt.validate(manifest)
    encoded = receipt.encode()
    write_receipt(home, manifest, receipt)
    return encoded
