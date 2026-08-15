"""Total Git normalization and recursive alias resolution for shell policy."""

from __future__ import annotations

import glob
import shlex
from collections.abc import Callable
from dataclasses import dataclass

from .repository import (
    UrlRewrite,
    git_aliases,
    git_directory_owned,
    git_url_rewrites,
    repository_owned,
    repository_owned_with_rewrites,
)
from .git_command_options import VALUE_OPTIONS, alias_option, option_value, url_rewrite
from .shell_context import resolve_cwd

NO_ARGUMENT_OPTIONS = {
    "-p",
    "-P",
    "--paginate",
    "--no-pager",
    "--bare",
    "--no-replace-objects",
    "--no-lazy-fetch",
    "--literal-pathspecs",
    "--glob-pathspecs",
    "--noglob-pathspecs",
    "--icase-pathspecs",
    "--no-optional-locks",
    "--help",
    "--version",
    "--exec-path",
    "--html-path",
    "--man-path",
    "--info-path",
}
MAX_ALIAS_DEPTH = 8


@dataclass(frozen=True)
class GitInvocation:
    operation: str | None
    arguments: list[str]
    cwd: str
    cwd_owned: bool | None
    git_dir: str | None
    alias_command: str | None = None
    rewrites: tuple[UrlRewrite, ...] = ()


def normalize(
    arguments: list[str],
    cwd: str,
    cwd_owned: bool | None,
    git_dir: str | None,
    config_owned: Callable[[str], bool],
    environment_config: dict[str, str],
    remote_urls: tuple[tuple[str, str, str], ...] = (),
) -> GitInvocation | None:
    """Return a policy-ready effective Git invocation, or fail closed."""
    try:
        aliases: dict[str, str] = {}
        rewrites: list[UrlRewrite] = []
        for key, value in environment_config.items():
            config = f"{key}={value}"
            alias = alias_option(config)
            is_url_config, rewrite = url_rewrite(config)
            if alias is not None:
                alias_name, command = alias
                aliases[alias_name] = command
            elif is_url_config:
                if rewrite is None:
                    return None
                rewrites.append(rewrite)
            elif cwd_owned is not False or config_owned(config):
                return None
        return _normalize(
            list(arguments),
            cwd,
            cwd_owned,
            git_dir,
            config_owned,
            aliases,
            rewrites,
            set(),
            0,
            remote_urls,
        )
    except (OSError, TypeError, ValueError):
        return None


def _normalize(
    arguments: list[str],
    cwd: str,
    cwd_owned: bool | None,
    git_dir: str | None,
    config_owned: Callable[[str], bool],
    inline_aliases: dict[str, str],
    rewrites: list[UrlRewrite],
    seen: set[str],
    depth: int,
    remote_urls: tuple[tuple[str, str, str], ...],
) -> GitInvocation | None:
    while arguments and arguments[0].startswith("-"):
        option = arguments.pop(0)
        if option == "--":
            break
        if option in NO_ARGUMENT_OPTIONS:
            continue
        name, value = option_value(option, arguments)
        if name not in VALUE_OPTIONS or value is None:
            return None
        if option == name:
            arguments.pop(0)
        if name == "-C":
            cwd = resolve_cwd(cwd, value)
            cwd_owned = (
                git_directory_owned(cwd, git_dir)
                if git_dir is not None
                else repository_owned(cwd)
            )
        elif name == "-c":
            alias = alias_option(value)
            is_url_config, rewrite = url_rewrite(value)
            if alias is not None:
                key, command = alias
                inline_aliases[key] = command
            elif is_url_config:
                if rewrite is None:
                    return None
                rewrites.append(rewrite)
            elif cwd_owned is not False or config_owned(value):
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
    if not arguments:
        return GitInvocation(
            None, [], cwd, cwd_owned, git_dir, rewrites=tuple(rewrites)
        )
    operation = _operation(arguments[0], cwd)
    if operation is None:
        return None
    rest = arguments[1:]
    if operation.casefold() == "clone":
        return GitInvocation(
            operation, rest, cwd, cwd_owned, git_dir, rewrites=tuple(rewrites)
        )
    push_like = operation.casefold() in {"push", "send-pack"}
    if rewrites or push_like:
        active_rewrites = git_url_rewrites(cwd, git_dir)
        if active_rewrites is None:
            return None
        rewrites = active_rewrites + rewrites
        cwd_owned = repository_owned_with_rewrites(
            cwd, git_dir, rewrites, push_like, remote_urls
        )
    alias_name = operation.casefold()
    aliases = git_aliases(cwd, git_dir)
    if aliases is None:
        return None
    aliases.update(inline_aliases)
    command = aliases.get(alias_name)
    if command is None:
        return GitInvocation(
            operation, rest, cwd, cwd_owned, git_dir, rewrites=tuple(rewrites)
        )
    if depth >= MAX_ALIAS_DEPTH or alias_name in seen:
        return None
    if command.lstrip().startswith("!"):
        return GitInvocation(
            None,
            [],
            cwd,
            cwd_owned,
            git_dir,
            command.lstrip()[1:].strip(),
            tuple(rewrites),
        )
    try:
        expanded = shlex.split(command, posix=True)
    except ValueError:
        return None
    if not expanded:
        return None
    return _normalize(
        expanded + rest,
        cwd,
        cwd_owned,
        git_dir,
        config_owned,
        inline_aliases,
        rewrites,
        seen | {alias_name},
        depth + 1,
        remote_urls,
    )


def _operation(value: str, cwd: str) -> str | None:
    """Expand one Git command-word glob, failing closed on ambiguous matches."""
    if not glob.has_magic(value):
        return value
    matches = glob.iglob(value, root_dir=cwd)
    first = next(matches, None)
    if first is None:
        return value
    return first if next(matches, None) is None else None
