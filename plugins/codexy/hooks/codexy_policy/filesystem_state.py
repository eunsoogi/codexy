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
    destination = location(arguments[0], cwd)
    current = state(destination, paths)
    if current.kind != "absent":
        return paths if parents and current.kind == "directory" else None
    additions = []
    cursor = destination
    while state(cursor, paths).kind == "absent":
        additions.append(cursor)
        cursor = str(Path(cursor).parent)
        if cursor == str(Path(cursor).parent):
            return None
    if state(cursor, paths).kind != "directory":
        return None
    if not parents and len(additions) != 1:
        return None
    indexed = dict(paths)
    for path in reversed(additions):
        indexed[path] = DIRECTORY
    return tuple(indexed.items())
