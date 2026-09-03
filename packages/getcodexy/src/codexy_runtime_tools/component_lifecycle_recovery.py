"""Durable recovery and host mutation helpers for component operations."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Callable

from .component_hook_activation import HookLister
from .component_lifecycle_finish import finish_committed
from .component_lifecycle_preflight import existing_marketplace
from .component_manifest import ComponentManifest
from .component_resolver import (
    ComponentResolutionError,
    reconcile_installed_inventory,
    verify_post_operation_inventory,
)
from .component_transaction_state import (
    InventorySnapshot,
    Journal,
    clear_journal,
    decode_inventory,
    read_journal,
    write_inventory,
    write_journal,
)
from .component_lifecycle_terminal import terminal
from .marketplace_repin import reconcile_official_marketplace_root
from .pre_session import _json, official_marketplace_root
from .plugin_resolution import MarketplaceBinding, MarketplaceIdentity


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]


def recover_if_needed(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    hook_lister: HookLister | None = None,
) -> None:
    journal = read_journal(home)
    if journal is None:
        return
    journal.validate(manifest, decode_inventory)
    if journal.phase == "committed":
        finish_committed(
            home,
            executable,
            invoke,
            manifest,
            root,
            journal,
            hook_lister,
            list_installed,
            clear_journal,
        )
        return
    if journal.command in {"update", "bootstrap"} and journal.phase == "started":
        try:
            installed = apply_forward(
                executable, invoke, manifest, root, journal, journal.resolved, (), home
            )
        except BaseException as error:
            rollback_or_raise(home, executable, invoke, manifest, root, journal, error)
            terminal(home, manifest, journal.receipt("rolled-back", journal.before))
            clear_journal(home)
            return
        write_completed(
            home, executable, invoke, manifest, root, journal, installed, hook_lister
        )
        return
    if journal.phase == "started":
        try:
            installed = verify_post_operation_inventory(
                manifest, list_installed(executable, invoke), journal.target, root
            )
        except ComponentResolutionError:
            installed = None
        if installed is not None:
            write_completed(
                home,
                executable,
                invoke,
                manifest,
                root,
                journal,
                installed,
                hook_lister,
            )
            return
    rollback_or_raise(
        home,
        executable,
        invoke,
        manifest,
        root,
        journal,
        RuntimeError("interrupted component operation"),
    )
    terminal(home, manifest, journal.receipt("rolled-back", journal.before))
    clear_journal(home)


def rollback_or_raise(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
    cause: BaseException,
) -> None:
    try:
        write_journal(home, journal.with_phase("rolling-back"))
        restore_selection(executable, invoke, manifest, root, journal.before)
        if (
            selection(manifest, list_installed(executable, invoke), root)
            != journal.before
        ):
            raise RuntimeError(
                "restored selection did not match the operation snapshot"
            )
        journal.snapshot.restore(home)
        if InventorySnapshot.capture(home) != journal.snapshot:
            raise RuntimeError(
                "restored durable inventory did not match the operation snapshot"
            )
    except BaseException as rollback_error:
        raise RuntimeError(
            "component operation failed; durable recovery is required"
        ) from rollback_error


def write_completed(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
    installed: tuple[str, ...],
    hook_lister: HookLister | None = None,
) -> dict[str, object]:
    write_inventory(home, installed)
    write_journal(home, journal.with_phase("committed"))
    return finish_committed(
        home,
        executable,
        invoke,
        manifest,
        root,
        journal,
        hook_lister,
        list_installed,
        clear_journal,
    )


def restore_selection(
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    before: tuple[str, ...],
) -> None:
    current = selection(manifest, list_installed(executable, invoke), root)
    for component in before:
        if component not in current:
            mutate(executable, invoke, "add", manifest, component)
    for component in reversed(manifest.component_ids):
        if component in current and component not in before:
            mutate(executable, invoke, "remove", manifest, component)


def apply_forward(
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
    adds: tuple[str, ...],
    removes: tuple[str, ...],
    home: Path,
) -> tuple[str, ...]:
    if journal.command in {"update", "bootstrap"}:
        if isinstance(root, MarketplaceIdentity) and root.source_type == "local":
            if existing_marketplace(executable, invoke, manifest) != root:
                raise ValueError("local Codexy marketplace identity changed")
        else:
            root = reconcile_official_marketplace_root(
                executable,
                invoke,
                manifest.version,
                home,
            )
    for component in adds:
        mutate(executable, invoke, "add", manifest, component)
    for component in removes:
        mutate(executable, invoke, "remove", manifest, component)
    return verify_post_operation_inventory(
        manifest, list_installed(executable, invoke), journal.target, root
    )


def mutate(
    executable: Path,
    invoke: Runner,
    action: str,
    manifest: ComponentManifest,
    component_id: str,
) -> None:
    _json(
        invoke(
            [
                str(executable),
                "plugin",
                action,
                manifest.component(component_id).asset.plugin_id,
                "--json",
            ]
        ),
        f"plugin {action}",
    )


def list_installed(executable: Path, invoke: Runner) -> object:
    return _json(invoke([str(executable), "plugin", "list", "--json"]), "plugin list")


def selection(
    manifest: ComponentManifest, payload: object, root: MarketplaceBinding
) -> tuple[str, ...]:
    return reconcile_installed_inventory(manifest, payload, root)
