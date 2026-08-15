"""Narrow, no-follow rollback snapshot for public component activation."""

from __future__ import annotations

import os
import stat
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Entry:
    relative: Path
    data: bytes | None
    mode: int


@dataclass(frozen=True)
class ActivationSnapshot:
    home: Path
    entries: tuple[Entry, ...]

    @classmethod
    def capture(cls, home: Path) -> "ActivationSnapshot":
        if not home.exists():
            return cls(home, ())
        files: list[Entry] = []
        for root in (
            Path("config.toml"),
            Path("agents/codexy"),
            Path("agents/codexy-github"),
        ):
            files.extend(_capture(home, root))
        for backup in home.glob("config.toml.codexy-backup-*"):
            files.extend(_capture(home, backup.relative_to(home)))
        return cls(home, tuple(files))

    def restore(self) -> None:
        for root in (
            Path("agents/codexy-github"),
            Path("agents/codexy"),
            Path("config.toml"),
        ):
            _remove_tree(self.home / root)
        if self.home.exists():
            for backup in self.home.glob("config.toml.codexy-backup-*"):
                _remove_tree(backup)
        if self.entries:
            self.home.mkdir(parents=True, exist_ok=True)
        for entry in sorted(self.entries, key=lambda item: len(item.relative.parts)):
            path = self.home / entry.relative
            if entry.data is None:
                path.mkdir(mode=entry.mode, exist_ok=True)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                descriptor = os.open(
                    path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, entry.mode
                )
                with os.fdopen(descriptor, "wb") as output:
                    output.write(entry.data)
        _remove_if_empty(self.home / "agents")
        _remove_if_empty(self.home)


def _capture(home: Path, relative: Path) -> list[Entry]:
    path = home / relative
    if not os.path.lexists(path):
        return []
    metadata = path.lstat()
    _require_safe(metadata, path)
    if stat.S_ISREG(metadata.st_mode):
        return [Entry(relative, _read_file(path, metadata), stat.S_IMODE(metadata.st_mode))]
    descriptor = _open_directory(path, metadata)
    try:
        return _capture_directory(relative, descriptor)
    finally:
        os.close(descriptor)


def _capture_directory(relative: Path, descriptor: int) -> list[Entry]:
    metadata = os.fstat(descriptor)
    _require_safe(metadata, relative)
    entries = [Entry(relative, None, stat.S_IMODE(metadata.st_mode))]
    for name in sorted(os.listdir(descriptor)):
        child = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
        child_relative = relative / name
        _require_safe(child, child_relative)
        if stat.S_ISREG(child.st_mode):
            entries.append(Entry(child_relative, _read_child(descriptor, name, child), stat.S_IMODE(child.st_mode)))
            continue
        child_descriptor = os.open(
            name,
            os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0),
            dir_fd=descriptor,
        )
        try:
            opened = os.fstat(child_descriptor)
            if (opened.st_dev, opened.st_ino) != (child.st_dev, child.st_ino):
                raise ValueError(f"activation state changed while reading: {child_relative}")
            entries.extend(_capture_directory(child_relative, child_descriptor))
        finally:
            os.close(child_descriptor)
    return entries


def _remove_tree(path: Path) -> None:
    if not os.path.lexists(path):
        return
    metadata = path.lstat()
    _require_safe(metadata, path)
    if stat.S_ISREG(metadata.st_mode):
        path.unlink()
        return
    for child in sorted(path.iterdir()):
        _remove_tree(child)
    path.rmdir()


def _remove_if_empty(path: Path) -> None:
    try:
        path.rmdir()
    except FileNotFoundError:
        pass
    except OSError:
        pass


def _read_file(path: Path, expected: os.stat_result) -> bytes:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        return _read_descriptor(descriptor, expected, path)
    finally:
        os.close(descriptor)


def _read_child(directory: int, name: str, expected: os.stat_result) -> bytes:
    descriptor = os.open(
        name, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0), dir_fd=directory
    )
    try:
        return _read_descriptor(descriptor, expected, Path(name))
    finally:
        os.close(descriptor)


def _read_descriptor(descriptor: int, expected: os.stat_result, path: Path) -> bytes:
    opened = os.fstat(descriptor)
    _require_safe(opened, path)
    if (opened.st_dev, opened.st_ino) != (expected.st_dev, expected.st_ino):
        raise ValueError(f"activation state changed while reading: {path}")
    with os.fdopen(descriptor, "rb", closefd=False) as source:
        return source.read()


def _open_directory(path: Path, expected: os.stat_result) -> int:
    descriptor = os.open(
        path, os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0)
    )
    opened = os.fstat(descriptor)
    if (opened.st_dev, opened.st_ino) != (expected.st_dev, expected.st_ino):
        os.close(descriptor)
        raise ValueError(f"activation state changed while reading: {path}")
    return descriptor


def _require_safe(metadata: os.stat_result, path: Path) -> None:
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    if stat.S_ISLNK(metadata.st_mode) or getattr(metadata, "st_file_attributes", 0) & reparse:
        raise ValueError(f"activation state refuses link: {path}")
    if stat.S_ISREG(metadata.st_mode) and metadata.st_nlink == 1:
        return
    if stat.S_ISDIR(metadata.st_mode):
        return
    raise ValueError(f"activation state requires real directories and regular files: {path}")
