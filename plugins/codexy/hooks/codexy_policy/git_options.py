"""Declarative long-option normalization for policy-sensitive Git commands."""

from __future__ import annotations

from typing import Literal

ValueMode = Literal["none", "required", "optional-equals"]

SCHEMAS: dict[str, dict[str, ValueMode]] = {
    "reset": {
        "hard": "none", "help": "none", "soft": "none", "mixed": "none",
        "merge": "none", "keep": "none", "quiet": "none", "refresh": "none",
        "no-refresh": "none", "intent-to-add": "none", "no-intent-to-add": "none",
        "recurse-submodules": "optional-equals", "no-recurse-submodules": "none",
        "pathspec-from-file": "required", "pathspec-file-nul": "none",
    },
    "clean": {
        "force": "none", "interactive": "none", "dry-run": "none",
        "quiet": "none", "exclude": "required", "help": "none",
    },
    "push": {
        "force": "none", "force-with-lease": "optional-equals",
        "force-if-includes": "none", "mirror": "none", "delete": "none",
        "prune": "none", "all": "none", "tags": "none", "dry-run": "none",
        "porcelain": "none", "quiet": "none", "verbose": "none",
        "set-upstream": "none", "atomic": "none", "follow-tags": "none",
        "signed": "optional-equals", "no-signed": "none", "push-option": "required",
        "receive-pack": "required", "repo": "required", "thin": "none",
        "no-thin": "none", "ipv4": "none", "ipv6": "none",
        "recurse-submodules": "required", "no-recurse-submodules": "none",
        "exec": "required", "progress": "none", "no-progress": "none",
        "help": "none",
    },
}


def normalize(operation: str, arguments: list[str]) -> list[str] | None:
    """Resolve unique long-option prefixes or reject unsupported command shapes."""
    schema = SCHEMAS.get(operation)
    if schema is None:
        return list(arguments)
    normalized: list[str] = []
    index = 0
    while index < len(arguments):
        token = arguments[index]
        if token == "--":
            return normalized + arguments[index:]
        if not token.startswith("--"):
            normalized.append(token)
            index += 1
            continue
        provided, separator, value = token[2:].partition("=")
        if provided in schema:
            name = provided
        else:
            matches = [candidate for candidate in schema if candidate.startswith(provided)]
            if len(matches) != 1:
                return None
            name = matches[0]
        mode = schema[name]
        if separator and mode == "none":
            return None
        normalized.append(f"--{name}" + (f"={value}" if separator else ""))
        if mode == "required" and not separator:
            if index + 1 >= len(arguments):
                return None
            normalized.append(arguments[index + 1])
            index += 1
        index += 1
    return normalized
