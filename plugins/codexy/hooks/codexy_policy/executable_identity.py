"""Resolve copied or linked GitHub and Git executables before admission."""

from __future__ import annotations

import hashlib
import os
import shutil
import stat
from functools import lru_cache
from pathlib import Path

from .shell_context import name

MAX_EXECUTABLE_BYTES = 64 * 1024 * 1024
SENSITIVE_EXECUTABLES = frozenset({"git", "gh"})


def resolve(command: str, cwd: str, aliases: tuple[tuple[str, str], ...] = ()) -> str:
    """Return a sensitive executable identity when one can be proven."""
    lexical = name(command)
    if lexical in SENSITIVE_EXECUTABLES:
        return lexical
    alias = dict(aliases).get(_location(command, cwd))
    if alias is not None:
        return alias
    candidate = _path(command, cwd)
    if candidate is None:
        return lexical
    for executable in SENSITIVE_EXECUTABLES:
        target = shutil.which(executable)
        if target is not None and _same_executable(candidate, Path(target)):
            return executable
    return lexical


def created_alias(
    executable: str, arguments: list[str], cwd: str, aliases: tuple[tuple[str, str], ...],
) -> tuple[str, str] | None:
    """Return a statically provable Git/GH destination created by ``ln`` or ``cp``."""
    operands = _alias_operands(executable, arguments)
    if operands is None:
        return None
    source, destination = operands
    identity = resolve(source, cwd, aliases)
    location = _location(destination, cwd)
    return (location, identity) if identity in SENSITIVE_EXECUTABLES and location is not None else None


def _alias_operands(executable: str, arguments: list[str]) -> tuple[str, str] | None:
    options = {"-s", "-f", "-sf", "-fs", "--symbolic", "--force"}
    while arguments and arguments[0] in options:
        arguments = arguments[1:]
    if arguments[:1] == ["--"]:
        arguments = arguments[1:]
    return (arguments[0], arguments[1]) if executable in {"ln", "cp"} and len(arguments) == 2 else None


def _location(command: str, cwd: str) -> str | None:
    if "/" not in command:
        return None
    path = Path(command)
    return str((path if path.is_absolute() else Path(cwd) / path).resolve(strict=False))


def _path(command: str, cwd: str) -> Path | None:
    if "/" not in command:
        found = shutil.which(command)
        return Path(found) if found is not None else None
    path = Path(command)
    path = path if path.is_absolute() else Path(cwd) / path
    try:
        return path.resolve(strict=True)
    except OSError:
        return None


def _same_executable(candidate: Path, target: Path) -> bool:
    try:
        if os.path.samefile(candidate, target):
            return True
        return _digest(candidate) == _digest(target)
    except OSError:
        return False


@lru_cache(maxsize=32)
def _digest(path: Path) -> bytes | None:
    try:
        metadata = path.stat()
        if not stat.S_ISREG(metadata.st_mode) or not metadata.st_mode & 0o111:
            return None
        if metadata.st_size > MAX_EXECUTABLE_BYTES:
            return None
        digest = hashlib.sha256()
        with path.open("rb") as source:
            while chunk := source.read(65536):
                digest.update(chunk)
        return digest.digest()
    except OSError:
        return None
