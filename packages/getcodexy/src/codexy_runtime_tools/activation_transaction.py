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
    if stat.S_ISLNK(metadata.st_mode):
        raise ValueError(f"activation state must not traverse a symlink: {path}")
    mode = stat.S_IMODE(metadata.st_mode)
    if stat.S_ISREG(metadata.st_mode):
        return [Entry(relative, path.read_bytes(), mode)]
    if not stat.S_ISDIR(metadata.st_mode):
        raise ValueError(f"activation state must contain regular files: {path}")
    entries = [Entry(relative, None, mode)]
    for child in sorted(path.iterdir()):
        entries.extend(_capture(home, child.relative_to(home)))
    return entries


def _remove_tree(path: Path) -> None:
    if not os.path.lexists(path):
        return
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode):
        raise ValueError(f"activation rollback refuses symlink: {path}")
    if stat.S_ISREG(metadata.st_mode):
        path.unlink()
        return
    if not stat.S_ISDIR(metadata.st_mode):
        raise ValueError(f"activation rollback refuses special path: {path}")
    for child in path.iterdir():
        _remove_tree(child)
    path.rmdir()


def _remove_if_empty(path: Path) -> None:
    try:
        path.rmdir()
    except FileNotFoundError:
        pass
    except OSError:
        pass
