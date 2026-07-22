"""Total Git normalization and recursive alias resolution for shell policy."""

from __future__ import annotations

import shlex
from collections.abc import Callable
from dataclasses import dataclass

from .repository import UrlRewrite, git_aliases, git_directory_owned, repository_owned, repository_owned_with_rewrites
from .shell_context import resolve_cwd

NO_ARGUMENT_OPTIONS = {
    "-p", "-P", "--paginate", "--no-pager", "--bare", "--no-replace-objects",
    "--no-lazy-fetch", "--literal-pathspecs", "--glob-pathspecs", "--noglob-pathspecs",
    "--icase-pathspecs", "--no-optional-locks", "--help", "--version", "--exec-path",
    "--html-path", "--man-path", "--info-path",
}
VALUE_OPTIONS = {"-C", "-c", "--git-dir", "--work-tree", "--namespace", "--super-prefix", "--config-env", "--exec-path"}
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
    arguments: list[str], cwd: str, cwd_owned: bool | None, git_dir: str | None,
    config_owned: Callable[[str], bool], environment_config: dict[str, str],
) -> GitInvocation | None:
    """Return a policy-ready effective Git invocation, or fail closed."""
    try:
        aliases: dict[str, str] = {}
        for key, value in environment_config.items():
            alias = _alias_option(f"{key}={value}")
            if alias is None:
                return None
            alias_name, command = alias
            aliases[alias_name] = command
        return _normalize(list(arguments), cwd, cwd_owned, git_dir, config_owned, aliases, [], set(), 0)
    except (OSError, TypeError, ValueError):
        return None


def _normalize(
    arguments: list[str], cwd: str, cwd_owned: bool | None, git_dir: str | None,
    config_owned: Callable[[str], bool], inline_aliases: dict[str, str], rewrites: list[UrlRewrite],
    seen: set[str], depth: int,
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
            alias = _alias_option(value)
            is_url_config, rewrite = _url_rewrite(value)
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
        return GitInvocation(None, [], cwd, cwd_owned, git_dir, rewrites=tuple(rewrites))
    operation, rest = arguments[0], arguments[1:]
    if rewrites:
        cwd_owned = repository_owned_with_rewrites(cwd, git_dir, rewrites, operation.casefold() == "push")
    alias_name = operation.casefold()
    aliases = git_aliases(cwd, git_dir)
    if aliases is None:
        return None
    aliases.update(inline_aliases)
    command = aliases.get(alias_name)
    if command is None:
        return GitInvocation(operation, rest, cwd, cwd_owned, git_dir, rewrites=tuple(rewrites))
    if depth >= MAX_ALIAS_DEPTH or alias_name in seen:
        return None
    if command.lstrip().startswith("!"):
        return GitInvocation(None, [], cwd, cwd_owned, git_dir, command.lstrip()[1:].strip(), tuple(rewrites))
    try:
        expanded = shlex.split(command, posix=True)
    except ValueError:
        return None
    if not expanded:
        return None
    return _normalize(expanded + rest, cwd, cwd_owned, git_dir, config_owned, inline_aliases, rewrites, seen | {alias_name}, depth + 1)


def _alias_option(value: str) -> tuple[str, str] | None:
    if "=" not in value:
        return None
    variable, command = value.split("=", 1)
    section, separator, key = variable.partition(".")
    if section.casefold() != "alias" or not separator:
        return None
    canonical = key.casefold()
    return (canonical, command) if canonical and all(part and part.replace("_", "").isalnum() for part in canonical.split(".")) else None


def _url_rewrite(value: str) -> tuple[bool, UrlRewrite | None]:
    variable, separator, prefix = value.partition("=")
    canonical = variable.casefold()
    if not canonical.startswith("url."):
        return False, None
    if not separator or not prefix or any(char in value for char in "\0\r\n"):
        return True, None
    if canonical.endswith(".pushinsteadof"):
        replacement = variable[4 : -len(".pushinsteadof")]
        push_only = True
    elif canonical.endswith(".insteadof"):
        replacement = variable[4 : -len(".insteadof")]
        push_only = False
    else:
        return True, None
    return True, UrlRewrite(prefix, replacement, push_only) if replacement else None


def _option_value(option: str, arguments: list[str]) -> tuple[str, str | None]:
    if option in VALUE_OPTIONS:
        return option, arguments[0] if arguments else None
    for name in ("--git-dir", "--work-tree", "--namespace", "--super-prefix", "--config-env", "--exec-path"):
        if option.startswith(name + "="):
            return name, option[len(name) + 1:]
    for name in ("-C", "-c"):
        if option.startswith(name) and len(option) > len(name):
            return name, option[len(name):].removeprefix("=")
    return option, None
