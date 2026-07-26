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
    applies: bool | None


@dataclass(frozen=True)
class AliasOperands:
    source: str
    destination: str
    symbolic: bool
    force: bool
    no_dereference: bool
    executable: str


def resolve(command: str, cwd: str, aliases: tuple[tuple[str, str | None], ...] = (), path: str | None = None) -> str:
    """Return a sensitive executable identity when one can be proven."""
    lexical = name(command)
    if lexical in SENSITIVE_EXECUTABLES:
        return lexical
    indexed = dict(aliases)
    for location in _command_locations(command, cwd, path):
        if location in indexed:
            return indexed[location] or lexical
        candidate = _path(location, cwd)
        if candidate is not None and _executable(candidate):
            return _identity(candidate) or lexical
    candidate = _path(command, cwd, path)
    if candidate is None:
        return lexical
    return _identity(candidate) or lexical


def alias_transition(
    executable: str, arguments: list[str], cwd: str, aliases: tuple[tuple[str, str | None], ...],
    directories: tuple[str, ...] = (),
) -> AliasTransition | None:
    """Parse one supported ``ln``/``cp`` transition, or reject ambiguity."""
    operands = _alias_operands(executable, arguments)
    if operands is None:
        return None
    identity, known = _filesystem_identity(operands.source, cwd, aliases)
    destination = _final_destination(operands, cwd, directories)
    if destination is None:
        return None
    return AliasTransition(destination, identity, known, _effect(operands, destination, aliases, directories))


def _alias_operands(executable: str, arguments: list[str]) -> AliasOperands | None:
    grammar = {
        "ln": (frozenset("sfnv"), frozenset({"--symbolic", "--force", "--no-dereference", "--verbose"})),
        "cp": (frozenset("pfv"), frozenset({"--preserve", "--force", "--verbose"})),
    }.get(executable)
    if grammar is None:
        return None
    short, long = grammar
    selected = set()
    while arguments and arguments[0].startswith("-") and arguments[0] != "-":
        option = arguments[0]
        if option == "--":
            arguments = arguments[1:]
            break
        if option.startswith("--"):
            if option not in long:
                return None
            selected.add(option)
        elif not option[1:] or not set(option[1:]) <= short:
            return None
        else:
            selected.update(option[1:])
        arguments = arguments[1:]
    if len(arguments) != 2 or not all(arguments):
        return None
    return AliasOperands(
        arguments[0], arguments[1], "s" in selected or "--symbolic" in selected,
        "f" in selected or "--force" in selected,
        "n" in selected or "--no-dereference" in selected, executable,
    )


def _command_locations(command: str, cwd: str, path: str | None) -> tuple[str, ...]:
    if "/" in command:
        return (_filesystem_location(command, cwd),)
    if path is None:
        return ()
    return tuple(_filesystem_location(os.path.join(directory or ".", command), cwd) for directory in path.split(os.pathsep))


def _filesystem_location(value: str, cwd: str) -> str:
    return os.path.abspath(os.path.normpath(os.path.join(cwd, value)))


def _final_destination(operands: AliasOperands, cwd: str, directories: tuple[str, ...]) -> str | None:
    result = _filesystem_location(operands.destination, cwd)
    if operands.no_dereference or not _directory_exists(result, directories):
        return result
    basename = Path(operands.source).name
    return str(Path(result) / basename) if basename not in {"", ".", ".."} else None


def _effect(
    operands: AliasOperands, destination: str, aliases: tuple[tuple[str, str | None], ...], directories: tuple[str, ...],
) -> bool | None:
    if not _directory_exists(str(Path(destination).parent), directories):
        return False
    exists = destination in dict(aliases) or Path(destination).exists()
    if operands.executable == "cp":
        return True if operands.force or not exists else None
    if operands.symbolic:
        return operands.force or not exists
    return False if exists else None


def _filesystem_identity(
    value: str, cwd: str, aliases: tuple[tuple[str, str | None], ...],
) -> tuple[str | None, bool]:
    location = _filesystem_location(value, cwd)
    indexed = dict(aliases)
    if location in indexed:
        return indexed[location], True
    candidate = _path(location, cwd)
    if candidate is None:
        return None, False
    return _identity(candidate), True


def _identity(candidate: Path) -> str | None:
    for executable in SENSITIVE_EXECUTABLES:
        target = shutil.which(executable)
        if target is not None and _same_executable(candidate, Path(target)):
            return executable
    return None


def directory_location(value: str, cwd: str) -> str:
    """Normalize a modeled directory path for later filesystem effects."""
    return _filesystem_location(value, cwd)


def directory_exists(value: str, directories: tuple[str, ...]) -> bool:
    """Return real or modeled directory existence without treating regular files as directories."""
    return _directory_exists(value, directories)


def _directory_exists(value: str, directories: tuple[str, ...]) -> bool:
    return value in directories or Path(value).is_dir()


def _executable(candidate: Path) -> bool:
    try:
        metadata = candidate.stat()
        return stat.S_ISREG(metadata.st_mode) and bool(metadata.st_mode & 0o111)
    except OSError:
        return False


def _path(command: str, cwd: str, path: str | None = None) -> Path | None:
    if "/" not in command:
        found = shutil.which(command, path=path)
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
