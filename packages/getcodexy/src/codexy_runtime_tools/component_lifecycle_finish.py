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
from .component_transaction_durability import sync_parent_directory
from .component_manifest import ComponentManifest
from .component_resolver import verify_post_operation_inventory
from .component_transaction_state import clear_journal as _clear_journal
from .component_transaction_state import write_inventory
from .component_transaction_snapshot import (
    CORE_MANAGED_MARKER,
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
    if not os.path.lexists(home):
        return ()
    _validate_real_path(home, require_exists=True)
    metadata = home.lstat()
    if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError(f"transaction snapshot requires a real home directory: {home}")
    entries: list[ManagedFileSnapshot] = []
    agents_root = home / "agents" / "codexy"
    if os.path.lexists(agents_root):
        root_metadata = agents_root.lstat()
        if _is_link(root_metadata) or not stat.S_ISDIR(root_metadata.st_mode):
            raise ValueError(
                f"transaction snapshot requires a real agent directory: {agents_root}"
            )
        with os.scandir(agents_root) as directory:
            for item in sorted(directory, key=lambda entry: entry.name):
                if not item.name.endswith(".toml"):
                    continue
                path = Path(item.path)
                contents = _read_regular(path)
                if contents is not None and contents.startswith(CORE_MANAGED_MARKER):
                    entries.append(
                        ManagedFileSnapshot(
                            Path("agents") / "codexy" / item.name,
                            contents,
                            stat.S_IMODE(path.lstat().st_mode),
                        )
                    )
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
    expected = {entry.relative: entry for entry in snapshots}
    agents_root = home / "agents" / "codexy"
    if os.path.lexists(agents_root):
        metadata = agents_root.lstat()
        if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(
                f"transaction restore requires a real agent directory: {agents_root}"
            )
        with os.scandir(agents_root) as directory:
            for item in sorted(directory, key=lambda entry: entry.name):
                if not item.name.endswith(".toml"):
                    continue
                path = Path(item.path)
                contents = _read_regular(path)
                relative = Path("agents") / "codexy" / item.name
                if contents is not None and contents.startswith(CORE_MANAGED_MARKER):
                    if relative not in expected:
                        _unlink_regular(path)
    for entry in snapshots:
        if entry.relative.parts[:2] == ("agents", "codexy"):
            _restore_managed_entry(home, entry)
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
    _remove_empty_directory(agents_root)
    _remove_empty_directory(agents_root.parent)


def _restore_managed_entry(home: Path, entry: ManagedFileSnapshot) -> None:
    path = home / entry.relative
    current = _read_regular(path)
    if current == entry.data:
        _set_mode(path, entry.mode)
        return
    if current is not None and entry.relative.parts[:2] == ("agents", "codexy"):
        if not current.startswith(CORE_MANAGED_MARKER):
            raise RuntimeError(f"managed restore refuses unmanaged path: {path}")
    _atomic_write(path, entry.data)
    _set_mode(path, entry.mode)


def _restore_managed_config(home: Path, entry: ManagedFileSnapshot) -> None:
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


def _remove_empty_directory(path: Path) -> None:
    if not os.path.lexists(path):
        return
    metadata = path.lstat()
    if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError(f"transaction restore requires a real directory: {path}")
    try:
        path.rmdir()
        sync_parent_directory(path.parent)
    except OSError:
        return


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
