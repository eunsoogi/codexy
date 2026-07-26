"""Bounded path-kind state for same-command executable admission."""

from __future__ import annotations

import os
import stat
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class PathState:
    kind: str
    identity: str | None = None


ABSENT = PathState("absent")
DIRECTORY = PathState("directory")


def location(value: str, cwd: str) -> str:
    """Return a canonical lookup key; mkdir effects retain lexical segments."""
    return os.path.abspath(os.path.normpath(os.path.join(cwd, value)))


def state(value: str, paths: tuple[tuple[str, PathState], ...]) -> PathState:
    if value in (indexed := dict(paths)):
        return indexed[value]
    path = Path(value)
    try:
        metadata = path.stat()
    except OSError:
        return ABSENT
    if stat.S_ISDIR(metadata.st_mode):
        return DIRECTORY
    return PathState("executable" if stat.S_ISREG(metadata.st_mode) and metadata.st_mode & 0o111 else "regular")


def mkdir(arguments: list[str], cwd: str, paths: tuple[tuple[str, PathState], ...]) -> tuple[tuple[str, PathState], ...] | None:
    parents = False
    while arguments[:1] and arguments[0].startswith("-"):
        option = arguments.pop(0)
        if option == "--":
            break
        if option not in {"-p", "--parents"}:
            return None
        parents = True
    if len(arguments) != 1 or not arguments[0]:
        return None
    return _mkdir_trace(arguments[0], cwd, paths, parents)


def _mkdir_trace(value: str, cwd: str, paths: tuple[tuple[str, PathState], ...], parents: bool) -> tuple[tuple[str, PathState], ...] | None:
    """Trace mkdir operands lexically: ``x/../y`` creates x before visiting y."""
    source = value if os.path.isabs(value) else os.path.join(cwd, value)
    segments = [segment for segment in source.split(os.path.sep) if segment not in {"", "."}]
    if not segments:
        return paths if parents and state(location(value, cwd), paths).kind == "directory" else None
    indexed = dict(paths)
    cursor = os.path.sep
    created = []
    for index, segment in enumerate(segments):
        if segment == "..":
            if _symlink_ambiguous(cursor, indexed) or state(cursor, tuple(indexed.items())).kind != "directory":
                return None
            cursor = str(Path(cursor).parent)
            continue
        cursor = os.path.join(cursor, segment)
        current = state(cursor, tuple(indexed.items()))
        if current.kind == "absent":
            if not parents and index != len(segments) - 1:
                return None
            indexed[cursor] = DIRECTORY
            created.append(cursor)
        elif current.kind != "directory":
            return None
    destination = location(value, cwd)
    if state(destination, tuple(indexed.items())).kind != "directory":
        return None
    if not parents and created != [destination]:
        return None
    return tuple(indexed.items())


def _symlink_ambiguous(path: str, paths: dict[str, PathState]) -> bool:
    return path not in paths and Path(path).is_symlink()
