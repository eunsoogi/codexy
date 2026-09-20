"""Opaque durable local-inventory snapshot used by lifecycle transitions."""

from __future__ import annotations

import os
import stat
from dataclasses import dataclass
from pathlib import Path

from .component_transaction_durability import sync_parent_directory
from .updater import _absolute, _validate_real_path


CORE_MANAGED_MARKER = b"# CODEXY MANAGED AGENT\n"
MANAGED_CONFIG_BEGIN = b"# BEGIN CODEXY MANAGED AGENTS"
MANAGED_CONFIG_END = b"# END CODEXY MANAGED AGENTS"


@dataclass(frozen=True)
class ManagedFileSnapshot:
    """One file owned by the core standalone-agent projection."""

    relative: Path
    data: bytes
    mode: int

    def __post_init__(self) -> None:
        relative = self.relative
        if isinstance(relative, str):
            relative = Path(relative)
            object.__setattr__(self, "relative", relative)
        if not isinstance(relative, Path) or not _allowed_path(relative):
            raise ValueError("component transaction has an unsafe managed snapshot")
        if not isinstance(self.data, bytes) or type(self.mode) is not int:
            raise ValueError("component transaction has an invalid managed snapshot")
        if not 0 <= self.mode <= 0o777:
            raise ValueError("component transaction has an invalid managed mode")
        if relative.parts[:2] == ("agents", "codexy") and not self.data.startswith(
            CORE_MANAGED_MARKER
        ):
            raise ValueError("component transaction has an unmanaged role snapshot")
        if relative == Path("config.toml") and not (
            MANAGED_CONFIG_BEGIN in self.data and MANAGED_CONFIG_END in self.data
        ):
            raise ValueError("component transaction has an unmanaged config snapshot")


@dataclass(frozen=True, eq=False)
class InventorySnapshot:
    contents: bytes | None
    managed_files: tuple[ManagedFileSnapshot, ...] | None = None

    def __post_init__(self) -> None:
        if self.contents is not None and not isinstance(self.contents, bytes):
            raise ValueError("component transaction has invalid inventory snapshot")
        if self.managed_files is not None:
            entries = tuple(self.managed_files)
            if any(not isinstance(entry, ManagedFileSnapshot) for entry in entries):
                raise ValueError(
                    "component transaction has an invalid managed snapshot"
                )
            if len({entry.relative for entry in entries}) != len(entries):
                raise ValueError(
                    "component transaction has duplicate managed snapshot paths"
                )
            object.__setattr__(self, "managed_files", entries)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, InventorySnapshot):
            return NotImplemented
        if self.contents != other.contents:
            return False
        # Journals written before managed projections were added have no
        # managed-file field. Their inventory is still authoritative, but
        # they cannot make claims about files they never captured.
        return (
            self.managed_files is None
            or other.managed_files is None
            or self.managed_files == other.managed_files
        )

    def __hash__(self) -> int:
        return hash(self.contents)

    @classmethod
    def capture(cls, home: object) -> "InventorySnapshot":
        from .component_transaction_state import capture_inventory_snapshot

        return capture_inventory_snapshot(home)

    def restore(self, home: object) -> None:
        from .component_transaction_state import restore_inventory_snapshot

        restore_inventory_snapshot(home, self)


def _allowed_path(relative: Path) -> bool:
    if relative.is_absolute() or any(
        part in {"", ".", ".."} for part in relative.parts
    ):
        return False
    parts = relative.parts
    return (
        relative == Path("config.toml")
        or (
            len(parts) == 3
            and parts[:2] == ("agents", "codexy")
            and parts[2].endswith(".toml")
            and not parts[2].startswith(".")
        )
        or (len(parts) == 1 and relative.name.startswith("config.toml.codexy-backup-"))
    )


def strip_managed_config(contents: bytes) -> bytes:
    """Remove only the registrar block outside TOML multiline strings."""
    from .component_transaction_state import _multiline_state

    try:
        text = contents.decode("utf-8")
    except UnicodeDecodeError:
        return contents
    lines = text.splitlines(keepends=True)
    kept: list[str] = []
    multiline: str | None = None
    in_block = found = False
    begin = MANAGED_CONFIG_BEGIN.decode()
    end = MANAGED_CONFIG_END.decode()
    for line in lines:
        marker = line.rstrip("\r\n")
        if multiline is None and marker == begin:
            if in_block:
                return contents
            in_block = found = True
            continue
        if multiline is None and marker == end:
            if not in_block:
                return contents
            in_block = False
            continue
        if not in_block:
            kept.append(line)
        multiline, _ = _multiline_state(line, multiline)
    if in_block:
        return contents
    return "".join(kept).encode() if found else contents


def _read_regular(target: Path) -> bytes | None:
    if not os.path.lexists(target.parent):
        _validate_real_path(target.parent, require_exists=False)
        return None
    _validate_real_path(target.parent, require_exists=True)
    if not os.path.lexists(target):
        return None
    metadata = target.lstat()
    if _is_link(metadata) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"transaction storage refuses non-regular path: {target}")
    descriptor = os.open(target, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (opened.st_dev, opened.st_ino) != (
            metadata.st_dev,
            metadata.st_ino,
        ):
            raise ValueError(
                f"transaction storage path changed while reading: {target}"
            )
        return os.read(descriptor, opened.st_size)
    finally:
        os.close(descriptor)


def _atomic_write(target: Path, contents: bytes) -> None:
    _ensure_directory(target.parent)
    if os.path.lexists(target):
        _read_regular(target)
    from uuid import uuid4

    temporary = target.parent / f".{target.name}.{uuid4().hex}.tmp"
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(contents)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, target)
        sync_parent_directory(target.parent)
    except BaseException:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def _unlink_regular(
    target: Path, expected_identity: tuple[int, int] | None = None
) -> None:
    if not os.path.lexists(target):
        return
    if (
        expected_identity is not None
        and (
            target.lstat().st_dev,
            target.lstat().st_ino,
        )
        != expected_identity
    ):
        return
    _read_regular(target)
    if (
        expected_identity is not None
        and (
            target.lstat().st_dev,
            target.lstat().st_ino,
        )
        != expected_identity
    ):
        return
    target.unlink()
    sync_parent_directory(target.parent)


def _ensure_directory(target: Path) -> None:
    absolute = _absolute(target)
    _validate_real_path(absolute, require_exists=False)
    current = Path(absolute.anchor)
    for part in absolute.parts[1:]:
        current /= part
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            try:
                current.mkdir(mode=0o700)
            except FileExistsError:
                pass
            metadata = current.lstat()
        if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(
                f"transaction storage requires a real directory: {current}"
            )


def _is_link(metadata: os.stat_result) -> bool:
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return stat.S_ISLNK(metadata.st_mode) or bool(
        getattr(metadata, "st_file_attributes", 0) & reparse
    )
