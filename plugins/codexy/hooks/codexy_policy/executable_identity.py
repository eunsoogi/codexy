"""Resolve copied or linked GitHub and Git executables before admission."""

from __future__ import annotations

import hashlib
import os
import shutil
import stat
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path

from .filesystem_state import PathState, location as filesystem_location, resolved_location, state as path_state
from .shell_context import name

MAX_EXECUTABLE_BYTES = 64 * 1024 * 1024
SENSITIVE_EXECUTABLES = frozenset({"git", "gh"})


@dataclass(frozen=True)
class AliasTransition:
    destination: str
    state: PathState
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


def resolve(command: str, cwd: str, aliases: tuple[tuple[str, PathState], ...] = (), path: str | None = None) -> str | None:
    """Return a sensitive executable identity when one can be proven."""
    lexical = name(command)
    if lexical in SENSITIVE_EXECUTABLES:
        return lexical
    indexed = dict(aliases)
    locations = _command_locations(command, cwd, path, aliases)
    if locations is None:
        return None
    for location in locations:
        if (modeled := indexed.get(location)) is not None:
            if modeled.kind == "executable":
                return modeled.identity or lexical
            continue
        candidate = _path(location, cwd)
        if candidate is not None and path_state(str(candidate), ()).kind == "executable":
            return _identity(candidate) or lexical
    candidate = _path(command, cwd, path)
    if candidate is None:
        return lexical
    return _identity(candidate) or lexical


def alias_transition(
    executable: str, arguments: list[str], cwd: str, aliases: tuple[tuple[str, PathState], ...],
) -> AliasTransition | None:
    """Parse one supported ``ln``/``cp`` transition, or reject ambiguity."""
    operands = _alias_operands(executable, arguments)
    if operands is None:
        return None
    source, known = _filesystem_state(operands.source, cwd, aliases)
    destination = _final_destination(operands, cwd, aliases)
    if destination is None:
        return None
    target = filesystem_location(operands.source, str(Path(destination).parent)) if operands.symbolic else None
    result = PathState(source.kind, source.identity, operands.symbolic, target)
    return AliasTransition(destination, result, known, _effect(operands, destination, aliases))


def _alias_operands(executable: str, arguments: list[str]) -> AliasOperands | None:
    grammar = {
        "ln": (frozenset("sfnv"), frozenset({"--symbolic", "--force", "--no-dereference", "--verbose"})),
        "cp": (frozenset("Ppfv"), frozenset({"--preserve", "--force", "--no-dereference", "--verbose"})),
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
        "n" in selected or "P" in selected or "--no-dereference" in selected, executable,
    )


def _command_locations(command: str, cwd: str, path: str | None, aliases: tuple[tuple[str, PathState], ...]) -> tuple[str, ...] | None:
    if "/" in command:
        location = resolved_location(command, cwd, aliases)
        return None if location is None else (location,)
    if path is None:
        return ()
    locations = tuple(resolved_location(os.path.join(directory or ".", command), cwd, aliases) for directory in path.split(os.pathsep))
    return None if any(location is None for location in locations) else tuple(location for location in locations if location is not None)


def _final_destination(operands: AliasOperands, cwd: str, aliases: tuple[tuple[str, PathState], ...]) -> str | None:
    result = resolved_location(operands.destination, cwd, aliases, follow_final=not operands.no_dereference)
    if result is None:
        return None
    if operands.no_dereference or path_state(result, aliases).kind != "directory":
        return result
    basename = Path(operands.source).name
    return str(Path(result) / basename) if basename not in {"", ".", ".."} else None


def _effect(
    operands: AliasOperands, destination: str, aliases: tuple[tuple[str, PathState], ...],
) -> bool | None:
    if path_state(str(Path(destination).parent), aliases).kind != "directory":
        return False
    exists = path_state(destination, aliases).kind != "absent"
    if operands.executable == "cp":
        return True if operands.force or not exists else None
    if operands.symbolic:
        return operands.force or not exists
    return False if exists else None


def _filesystem_state(value: str, cwd: str, aliases: tuple[tuple[str, PathState], ...]) -> tuple[PathState, bool]:
    location = resolved_location(value, cwd, aliases)
    if location is None:
        return PathState("absent"), False
    indexed = dict(aliases)
    if location in indexed:
        return indexed[location], True
    candidate = _path(location, cwd)
    if candidate is None:
        return PathState("absent"), False
    real = path_state(str(candidate), ())
    return PathState(real.kind, _identity(candidate) if real.kind == "executable" else None), True


def _identity(candidate: Path) -> str | None:
    for executable in SENSITIVE_EXECUTABLES:
        target = shutil.which(executable)
        if target is not None and _same_executable(candidate, Path(target)):
            return executable
    return None


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
