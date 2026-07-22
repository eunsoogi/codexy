"""Total Git global-option normalization for shell policy evaluation."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from .repository import git_directory_owned, repository_owned
from .shell_context import resolve_cwd

NO_ARGUMENT_OPTIONS = {
    "-p", "-P", "--paginate", "--no-pager", "--bare", "--no-replace-objects",
    "--no-lazy-fetch", "--literal-pathspecs", "--glob-pathspecs", "--noglob-pathspecs",
    "--icase-pathspecs", "--no-optional-locks", "--help", "--version", "--exec-path",
    "--html-path", "--man-path", "--info-path",
}
VALUE_OPTIONS = {"-C", "-c", "--git-dir", "--work-tree", "--namespace", "--super-prefix", "--config-env", "--exec-path"}


@dataclass(frozen=True)
class GitInvocation:
    operation: str | None
    arguments: list[str]
    cwd_owned: bool | None
    alias_command: str | None = None


def normalize(
    arguments: list[str],
    cwd: str,
    cwd_owned: bool | None,
    git_dir: str | None,
    config_owned: Callable[[str], bool],
) -> GitInvocation | None:
    """Return a policy-ready invocation, or ``None`` for unsupported composition."""
    try:
        return _normalize(list(arguments), cwd, cwd_owned, git_dir, config_owned)
    except (OSError, TypeError, ValueError):
        return None


def _normalize(
    arguments: list[str],
    cwd: str,
    cwd_owned: bool | None,
    git_dir: str | None,
    config_owned: Callable[[str], bool],
) -> GitInvocation | None:
    while arguments and arguments[0].startswith("-"):
        option = arguments.pop(0)
        if option == "--":
            break
        if option in NO_ARGUMENT_OPTIONS:
            continue
        name, value = _option_value(option, arguments)
        if name not in VALUE_OPTIONS or value is None:
            return None
        if option == name:
            arguments.pop(0)
        if name == "-C":
            cwd = resolve_cwd(cwd, value)
            cwd_owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
        elif name == "-c":
            if value.startswith("alias.") and "=!" in value:
                return GitInvocation(None, [], cwd_owned, value.split("=!", 1)[1])
            if cwd_owned is not False or config_owned(value):
                return None
        elif name == "--git-dir":
            git_dir = value
            cwd_owned = git_directory_owned(cwd, git_dir)
        elif name == "--work-tree":
            resolve_cwd(cwd, value)
        elif name == "--config-env":
            return None
    if git_dir is not None:
        cwd_owned = git_directory_owned(cwd, git_dir)
    return GitInvocation(arguments[0] if arguments else None, arguments[1:], cwd_owned)


def _option_value(option: str, arguments: list[str]) -> tuple[str, str | None]:
    if option in VALUE_OPTIONS:
        return option, arguments[0] if arguments else None
    for name in ("--git-dir", "--work-tree", "--namespace", "--super-prefix", "--config-env", "--exec-path"):
        if option.startswith(name + "="):
            return name, option[len(name) + 1 :]
    for name in ("-C", "-c"):
        if option.startswith(name) and len(option) > len(name):
            return name, option[len(name) :].removeprefix("=")
    return option, None
