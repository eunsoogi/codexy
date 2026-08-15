from __future__ import annotations

import os
from pathlib import Path

from .component_lifecycle_admission import (
    admit_pending_receipt,
    admitted_bootstrap_recovery_selection,
    admitted_recovery_selection,
    admitted_selection,
    matching_receipt,
    replay_receipt,
)
from .component_manifest import ComponentManifest, load_component_manifest
from .component_lifecycle_preflight import (
    existing_marketplace_root,
    recorded_selection,
    validate_request,
)
from .component_lifecycle_recovery import (
    apply_forward as _apply_forward,
    list_installed as _list,
    recover_if_needed as _recover_if_needed,
    rollback_or_raise as _rollback_or_raise,
    write_completed as _write_completed,
)
from .component_lifecycle_terminal import reject as _reject, terminal as _terminal
from .component_lifecycle_support import (
    HostExecutableError,
    Runner,
    host_executable,
    operation_identifier,
)
from .component_resolver import (
    ComponentResolutionError,
    reconcile_installed_inventory,
    verify_post_operation_inventory,
)
from .component_transaction_receipts import write_receipt
from .component_transaction_state import (
    InventorySnapshot,
    Journal,
    PreAdmissionError,
    clear_journal,
    decode_inventory,
    inventory_path,
    read_journal,
    transaction_lock,
    write_inventory,
    write_journal,
)
from .component_transition_model import (
    OperationReceipt,
    Rejection,
    RejectionStage,
    StateFailure,
    plan_transition,
)
from .pre_session import _run, official_marketplace_root
from .updater import _absolute, _validate_real_path


def run_operation(
    command: str,
    requested: tuple[str, ...],
    codex_home: str | os.PathLike[str],
    codex: Path | None = None,
    runner: Runner | None = None,
    *,
    operation_id: str | None = None,
) -> dict[str, object]:
    """Run a serialized operation, recovering any preceding interrupted operation first."""
    if command not in {"install", "update", "remove", "bootstrap"}:
        raise ValueError(f"unsupported component operation: {command}")
    try:
        home = _absolute(codex_home)
        _validate_real_path(home, require_exists=False)
    except (OSError, RuntimeError, ValueError) as error:
        raise PreAdmissionError(str(error)) from error
    executable, invoke = (
        host_executable(codex),
        runner or (lambda args: _run(args, home)),
    )
    manifest, identifier = load_component_manifest(), operation_identifier(operation_id)
    with transaction_lock(home):
        pending = read_journal(home)
        if pending is not None:
            pending.validate(manifest, decode_inventory)
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
            return _reject(
                home,
                manifest,
                identifier,
                command,
                requested,
                (),
                RejectionStage.REQUEST,
                error,
            )
        except (OSError, ValueError, RuntimeError):
            return _reject(
                home,
                manifest,
                identifier,
                command,
                requested,
                (),
                RejectionStage.PRESTATE,
                StateFailure.INCONSISTENT_INSTALLED_STATE,
            )
        try:
            root = existing_marketplace_root(executable, invoke)
            inventory = _list(executable, invoke)
            if (
                pending is not None
                and pending.phase == "started"
                and pending.command == "update"
            ):
                before = admitted_recovery_selection(
                    manifest, inventory, root, pending.target
                )
            elif (
                pending is not None
                and pending.phase == "started"
                and pending.command == "bootstrap"
            ):
                before = admitted_bootstrap_recovery_selection(
                    manifest, inventory, root, pending.before, pending.target
                )
            else:
                before = admitted_selection(manifest, inventory, root, command)
        except ComponentResolutionError as error:
            return _reject(
                home,
                manifest,
                identifier,
                command,
                requested,
                (),
                RejectionStage.HOST,
                error,
            )
        except (OSError, ValueError, RuntimeError):
            return _reject(
                home,
                manifest,
                identifier,
                command,
                requested,
                (),
                RejectionStage.HOST,
                StateFailure.INCONSISTENT_INSTALLED_STATE,
            )
        if pending is not None:
            if pending.phase == "rolling-back" and pending_receipt is not None:
                if (
                    before != pending.before
                    or InventorySnapshot.capture(home) != pending.snapshot
                ):
                    raise ValueError(
                        "pending transaction receipt does not match restored state"
                    )
                clear_journal(home)
                if replay is not None:
                    return replay
                pending = None
            if pending is not None:
                _recover_if_needed(
                    home,
                    executable,
                    invoke,
                    manifest,
                    root or official_marketplace_root(executable, invoke),
                )
            root = existing_marketplace_root(executable, invoke)
            inventory = _list(executable, invoke)
            before = admitted_selection(manifest, inventory, root, command)
            recorded = recorded_selection(home, manifest)
            replay = replay_receipt(home, manifest, identifier, command, requested)
            if replay is not None:
                return replay
        if recorded is not None and recorded != before and command != "bootstrap":
            return _reject(
                home,
                manifest,
                identifier,
                command,
                requested,
                before,
                RejectionStage.PRESTATE,
                StateFailure.INCONSISTENT_INSTALLED_STATE,
            )
        try:
            plan = plan_transition(manifest, command, requested, before, recorded)
        except ComponentResolutionError as error:
            return _reject(
                home,
                manifest,
                identifier,
                command,
                requested,
                before,
                RejectionStage.PLAN,
                error,
            )
        except (OSError, ValueError, RuntimeError):
            return _reject(
                home,
                manifest,
                identifier,
                command,
                requested,
                before,
                RejectionStage.PRESTATE,
                StateFailure.INCONSISTENT_INSTALLED_STATE,
            )

        journal = plan.journal(identifier, InventorySnapshot.capture(home))
        write_journal(home, journal)
        try:
            root = root or official_marketplace_root(executable, invoke)
            installed = _apply_forward(
                executable, invoke, manifest, root, journal, plan.adds, plan.removes
            )
        except BaseException as error:
            if root is None:
                raise RuntimeError(
                    "component operation failed; durable recovery is required"
                ) from error
            _rollback_or_raise(home, executable, invoke, manifest, root, journal, error)
            receipt = _terminal(
                home, manifest, journal.receipt("rolled-back", journal.before)
            )
            clear_journal(home)
            return receipt
        return _write_completed(
            home, executable, invoke, manifest, root, journal, installed
        )
