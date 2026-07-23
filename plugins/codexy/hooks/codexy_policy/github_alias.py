"""Resolve non-shell GitHub CLI aliases before mutation admission."""

from __future__ import annotations

import shlex
import subprocess

BUILTINS = {
    "alias", "api", "attestation", "auth", "browse", "cache", "codespace",
    "config", "extension", "gist", "issue", "label", "org", "pr", "project",
    "release", "repo", "ruleset", "run", "search", "secret", "ssh-key",
    "status", "variable", "workflow",
}
MAX_DEPTH = 8


def expand(arguments: list[str]) -> list[str] | None:
    """Return the effective gh arguments, or fail closed on alias ambiguity."""
    current, seen = list(arguments), set()
    for _ in range(MAX_DEPTH):
        if not current or current[0].casefold() in BUILTINS:
            return current
        aliases = _aliases()
        if aliases is None:
            return None
        name = current[0].casefold()
        command = aliases.get(name)
        if command is None:
            return current
        if name in seen or command.lstrip().startswith("!"):
            return None
        try:
            expanded = shlex.split(command, posix=True)
        except ValueError:
            return None
        if not expanded:
            return None
        current, seen = expanded + current[1:], seen | {name}
    return None


def _aliases() -> dict[str, str] | None:
    try:
        result = subprocess.run(
            ["gh", "alias", "list"], capture_output=True, check=False, timeout=1,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode != 0 or len(result.stdout) > 65536:
        return None
    aliases: dict[str, str] = {}
    try:
        for line in result.stdout.decode("utf-8", "strict").splitlines():
            name, separator, command = line.partition(":")
            canonical = name.strip().casefold()
            if not separator or not canonical or canonical in aliases or not command.strip():
                return None
            aliases[canonical] = command.strip()
    except UnicodeError:
        return None
    return aliases
