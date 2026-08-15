"""Sensitive shell builtin and utility classification."""

from __future__ import annotations

from .shell_context import flag


def hash_path_alias(args: list[str]) -> bool:
    """Return whether Bash hash arguments install an executable pathname."""
    return any(
        arg.startswith("-") and not arg.startswith("--") and "p" in arg[1:]
        for arg in args
    )


def rm_forbidden(args: list[str]) -> bool:
    targets = [arg for arg in args if not arg.startswith("-")]
    broad = {"/", "/*", "~", "$HOME", "${HOME}"}
    return (
        flag(args, "r", "--recursive")
        and flag(args, "f", "--force")
        and any(
            target in broad or target.rstrip("/").endswith("/..") for target in targets
        )
    )
