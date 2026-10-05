"""Sensitive shell builtin and utility classification."""

from __future__ import annotations

from pathlib import Path

from .execution_context_types import CommandEffect, ExecutionContext
from .shell_context import flag

CACHE_DIRECTORIES = frozenset(
    {"__pycache__", ".mypy_cache", ".pytest_cache", ".ruff_cache"}
)


def hash_path_alias(args: list[str]) -> bool:
    """Return whether Bash hash arguments install an executable pathname."""
    return any(
        arg.startswith("-") and not arg.startswith("--") and "p" in arg[1:]
        for arg in args
    )


def test_effect(arguments: list[str], context: ExecutionContext) -> CommandEffect:
    """Model only the exit-state effect of the supported ``test -e`` grammar."""
    if len(arguments) != 2 or arguments[0] != "-e":
        return CommandEffect(context, context)
    path = Path(arguments[1])
    candidate = path if path.is_absolute() else Path(context.cwd) / path
    return (
        CommandEffect(context) if candidate.exists() else CommandEffect(None, context)
    )


def rm_forbidden(args: list[str], cwd: str | None = None) -> bool:
    # Recursive force deletion is admitted only for named generated caches below a known repository cwd.
    targets = [arg for arg in args if not arg.startswith("-")]
    broad = {"/", "/*", "~", "$HOME", "${HOME}"}
    if not flag(args, "r", "--recursive") or not flag(args, "f", "--force"):
        return False
    if not targets or any(target in broad for target in targets):
        return True
    if cwd is None:
        return True
    return any(not _safe_cache_target(target, cwd) for target in targets)


def find_forbidden(args: list[str], cwd: str | None = None) -> bool:
    """Allow only a bounded find/prune/exec cleanup of generated caches."""
    if not any(argument in {"-delete", "-exec", "-execdir"} for argument in args):
        return False
    if cwd is None:
        return True
    index = 0
    roots: list[str] = []
    while index < len(args) and not args[index].startswith("-"):
        roots.append(args[index])
        index += 1
    if not roots or any(not _safe_path(root, cwd) for root in roots):
        return True
    if args[index : index + 2] != ["-type", "d"]:
        return True
    index += 2
    if index + 1 >= len(args) or args[index] != "-name":
        return True
    if args[index + 1] not in CACHE_DIRECTORIES:
        return True
    index += 2
    if index >= len(args) or args[index] != "-prune":
        return True
    index += 1
    if index >= len(args) or args[index] not in {"-exec", "-execdir"}:
        return True
    return not _safe_find_rm(args[index + 1 :])


def _safe_find_rm(args: list[str]) -> bool:
    """Require find -exec to invoke one recursive, forced rm over its selected cache directories."""
    options = args[1:-2]
    return (
        len(args) >= 4
        and args[0] == "rm"
        and args[-2:] == ["{}", "+"]
        and bool(options)
        and all(option.startswith("-") for option in options)
        and flag(options, "r", "--recursive")
        and flag(options, "f", "--force")
    )


def _safe_cache_target(target: str, cwd: str) -> bool:
    return target.rstrip("/").split("/")[-1] in CACHE_DIRECTORIES and _safe_path(
        target, cwd
    )


def _safe_path(value: str, cwd: str) -> bool:
    if (
        not value
        or value.startswith(("/", "~", "$"))
        or ".." in value.split("/")
        or any(char in value for char in "*?[]{}")
    ):
        return False
    try:
        root, candidate = Path(cwd).resolve(), (Path(cwd) / value).resolve()
    except OSError:
        return False
    return candidate == root or root in candidate.parents
