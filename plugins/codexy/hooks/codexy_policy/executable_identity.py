"""Resolve copied or linked GitHub and Git executables before admission."""

from __future__ import annotations

import hashlib
import os
import shutil
import stat
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path

from .shell_context import name

MAX_EXECUTABLE_BYTES = 64 * 1024 * 1024
SENSITIVE_EXECUTABLES = frozenset({"git", "gh"})


@dataclass(frozen=True)
class AliasTransition:
    destination: str
    identity: str | None
    known: bool


def resolve(command: str, cwd: str, aliases: tuple[tuple[str, str], ...] = ()) -> str:
    """Return a sensitive executable identity when one can be proven."""
    lexical = name(command)
    if lexical in SENSITIVE_EXECUTABLES:
        return lexical
    alias = dict(aliases).get(_command_location(command, cwd))
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


def alias_transition(
    executable: str, arguments: list[str], cwd: str, aliases: tuple[tuple[str, str], ...],
) -> AliasTransition | None:
    """Parse one supported ``ln``/``cp`` transition, or reject ambiguity."""
    operands = _alias_operands(executable, arguments)
    if operands is None:
        return None
    source, destination = operands
    identity, known = _filesystem_identity(source, cwd, aliases)
    return AliasTransition(_filesystem_location(destination, cwd), identity, known)


def _alias_operands(executable: str, arguments: list[str]) -> tuple[str, str] | None:
    grammar = {
        "ln": (frozenset("sfnv"), frozenset({"--symbolic", "--force", "--no-dereference", "--verbose"})),
        "cp": (frozenset("pfv"), frozenset({"--preserve", "--force", "--verbose"})),
    }.get(executable)
    if grammar is None:
        return None
    short, long = grammar
    while arguments and arguments[0].startswith("-") and arguments[0] != "-":
        option = arguments[0]
        if option == "--":
            arguments = arguments[1:]
            break
        if option.startswith("--"):
            if option not in long:
                return None
        elif not option[1:] or not set(option[1:]) <= short:
            return None
        arguments = arguments[1:]
    return (arguments[0], arguments[1]) if len(arguments) == 2 and all(arguments) else None


def _command_location(command: str, cwd: str) -> str | None:
    if "/" not in command:
        return None
    return _filesystem_location(command, cwd)


def _filesystem_location(value: str, cwd: str) -> str:
    path = Path(value)
    return str((path if path.is_absolute() else Path(cwd) / path).resolve(strict=False))


def _filesystem_identity(
    value: str, cwd: str, aliases: tuple[tuple[str, str], ...],
) -> tuple[str | None, bool]:
    location = _filesystem_location(value, cwd)
    alias = dict(aliases).get(location)
    if alias is not None:
        return alias, True
    candidate = _path(location, cwd)
    if candidate is None:
        return None, False
    for executable in SENSITIVE_EXECUTABLES:
        target = shutil.which(executable)
        if target is not None and _same_executable(candidate, Path(target)):
            return executable, True
    return None, True


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
