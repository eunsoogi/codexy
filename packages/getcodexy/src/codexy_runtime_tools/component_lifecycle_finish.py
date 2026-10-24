"""Finalize committed component operations with host activation readback."""

from __future__ import annotations

import os
import subprocess
import stat
from pathlib import Path
from typing import Callable

from .component_hook_activation import HookLister, activation_for_inventory
from .component_lifecycle_admission import (
    admit_pending_receipt,
    matching_receipt,
    replay_receipt,
)
from .component_lifecycle_terminal import terminal
from .component_manifest import ComponentManifest
from .component_registration_snapshot import (
    capture_managed_role_files,
    restore_managed_role_files,
)
from .component_registration_projection import (
    MANAGED_MARKERS,
    managed_component_for_path,
)
from .component_resolver import verify_post_operation_inventory
from .component_transaction_state import clear_journal as _clear_journal
from .component_transaction_state import write_inventory
from .component_transaction_snapshot import (
    MANAGED_CONFIG_BEGIN,
    MANAGED_CONFIG_END,
    ManagedFileSnapshot,
    _atomic_write,
    _is_link,
    _read_regular,
    _unlink_regular,
    strip_managed_config,
)
from .updater import _validate_real_path
from .component_transition_journal import Journal
from .plugin_resolution import MarketplaceBinding


Runner = Callable[[list[str]], subprocess.CompletedProcess[str]]
InstalledLister = Callable[[Path, Runner], object]


def capture_managed_files(home: Path) -> tuple[ManagedFileSnapshot, ...]:
    """Capture only marker-owned roles, managed config, and Codexy backups."""
    if not os.path.lexists(home):
        return ()
    _validate_real_path(home, require_exists=True)
    metadata = home.lstat()
    if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError(f"transaction snapshot requires a real home directory: {home}")
    entries = list(capture_managed_role_files(home))
    config = home / "config.toml"
    config_contents = _read_regular(config)
    if config_contents is not None and all(
        marker in config_contents
        for marker in (MANAGED_CONFIG_BEGIN, MANAGED_CONFIG_END)
    ):
        entries.append(
            ManagedFileSnapshot(
                Path("config.toml"),
                config_contents,
                stat.S_IMODE(config.lstat().st_mode),
            )
        )
    with os.scandir(home) as directory:
        for item in sorted(directory, key=lambda entry: entry.name):
            if not item.name.startswith("config.toml.codexy-backup-"):
                continue
            path = Path(item.path)
            contents = _read_regular(path)
            if contents is not None:
                entries.append(
                    ManagedFileSnapshot(
                        Path(item.name), contents, stat.S_IMODE(path.lstat().st_mode)
                    )
                )
    return tuple(sorted(entries, key=lambda entry: entry.relative.as_posix()))


def restore_managed_files(
    home: Path, snapshots: tuple[ManagedFileSnapshot, ...]
) -> None:
    """Restore the captured managed surface while leaving user-owned files alone."""
    expected = {entry.relative: entry for entry in snapshots}
    restore_managed_role_files(home, snapshots, _restore_managed_entry)
    config_entry = expected.get(Path("config.toml"))
    if config_entry is not None:
        _restore_managed_config(home, config_entry)
    original_config = config_entry.data if config_entry is not None else None
    for relative, entry in expected.items():
        if len(relative.parts) == 1 and relative.name.startswith(
            "config.toml.codexy-backup-"
        ):
            _restore_managed_entry(home, entry)
    _remove_new_backups(home, expected, original_config)


def _restore_managed_entry(home: Path, entry: ManagedFileSnapshot) -> None:
    """Restore one managed file, refusing to replace an unmarked agent file."""
    path = home / entry.relative
    current = _read_regular(path)
    if current == entry.data:
        _set_mode(path, entry.mode)
        return
    component = managed_component_for_path(entry.relative)
    if current is not None and component is not None:
        if not current.startswith(MANAGED_MARKERS[component].encode()):
            raise RuntimeError(f"managed restore refuses unmanaged path: {path}")
    _atomic_write(path, entry.data)
    _set_mode(path, entry.mode)


def _restore_managed_config(home: Path, entry: ManagedFileSnapshot) -> None:
    """Restore config only if remaining content matches the managed edit."""
    path = home / entry.relative
    current = _read_regular(path)
    if current == entry.data:
        _set_mode(path, entry.mode)
        return
    if current != strip_managed_config(entry.data):
        raise RuntimeError(f"managed restore refuses changed config: {path}")
    _atomic_write(path, entry.data)
    _set_mode(path, entry.mode)


def _remove_new_backups(
    home: Path,
    expected: dict[Path, ManagedFileSnapshot],
    original_config: bytes | None,
) -> None:
    if original_config is None or not os.path.lexists(home):
        return
    with os.scandir(home) as directory:
        for item in sorted(directory, key=lambda entry: entry.name):
            if not item.name.startswith("config.toml.codexy-backup-"):
                continue
            relative = Path(item.name)
            if relative in expected:
                continue
            if _read_regular(Path(item.path)) == original_config:
                _unlink_regular(Path(item.path))


def _set_mode(path: Path, mode: int) -> None:
    _read_regular(path)
    os.chmod(path, mode)


def finish_committed(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
    hook_lister: HookLister | None,
    list_installed: InstalledLister,
    clear: Callable[[Path], None] = _clear_journal,
) -> dict[str, object]:
    """Read back committed host state and persist the final inventory and receipt."""
    stored = admit_pending_receipt(home, manifest, journal)
    if stored is not None:
        clear(home)
        return stored
    inventory = list_installed(executable, invoke)
    installed = verify_post_operation_inventory(
        manifest, inventory, journal.target, root
    )
    activation = (
        activation_for_inventory(
            manifest, inventory, root, executable, home, hook_lister=hook_lister
        )
        if journal.command in {"install", "update", "bootstrap"}
        else {}
    )
    activation_errors = tuple(dict.fromkeys(activation.values()))
    outcome = "pending-action" if activation_errors else "completed"
    receipt = journal.receipt(outcome, installed, activation_errors)
    if matching_receipt(home, manifest, receipt.encode()):
        clear(home)
        return receipt.encode()
    if (
        replay_receipt(
            home, manifest, journal.identifier, journal.command, journal.requested
        )
        is not None
    ):
        raise ValueError(
            f"operation receipt conflicts with committed transaction: {journal.identifier}"
        )
    write_inventory(home, installed)
    encoded = terminal(home, manifest, receipt)
    clear(home)
    return encoded
