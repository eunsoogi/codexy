"""Typed shell environment state for effective mutation admission."""

from __future__ import annotations

import re
from dataclasses import replace

from .execution_context_types import CommandEffect, ExecutionContext
from .execution_filesystem import after_external_command
from .repository import git_directory_owned, repository_owned

VARIABLE_NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
VARIABLE_REFERENCE = re.compile(
    r"\$(?:\{(?P<braced>[A-Za-z_][A-Za-z0-9_]*)\}|(?P<plain>[A-Za-z_][A-Za-z0-9_]*))"
)
DYNAMIC_VALUE = "__codexy_command_substitution__"
SINGLE_QUOTED_DOLLAR = "\ue000"
POLICY_SELECTORS = {"GH_REPO", "GIT_DIR", "GIT_COMMON_DIR"}


def assignment(value: str) -> bool:
    return (
        "=" in value
        and not value.startswith("-")
        and VARIABLE_NAME.fullmatch(value.split("=", 1)[0]) is not None
    )


def assign(value: str, context: ExecutionContext) -> ExecutionContext:
    key, assigned = value.split("=", 1)
    expanded = expand(assigned, context)
    environment = dict(context.environment)
    if expanded is None or (key in POLICY_SELECTORS and DYNAMIC_VALUE in expanded):
        environment[key] = assigned
        return replace(
            context,
            environment=tuple(environment.items()),
            opaque_environment=True,
            opaque_repository_state=context.opaque_repository_state
            or key == "GIT_COMMON_DIR",
        )
    environment[key] = expanded
    git_dir = expanded if key == "GIT_DIR" else context.git_dir
    gh_repo = expanded if key == "GH_REPO" else context.gh_repo
    owned = (
        git_directory_owned(context.cwd, git_dir)
        if git_dir is not None
        else context.cwd_owned
    )
    return replace(
        context,
        cwd_owned=owned,
        git_dir=git_dir,
        gh_repo=gh_repo,
        environment=tuple(environment.items()),
        opaque_repository_state=context.opaque_repository_state
        or key == "GIT_COMMON_DIR",
    )


def leading_assignments(
    tokens: list[str], context: ExecutionContext
) -> tuple[list[str], ExecutionContext]:
    while tokens and assignment(tokens[0]):
        context = assign(tokens[0], context)
        tokens = tokens[1:]
    return tokens, context


def assigned_variables(
    arguments: list[str], context: ExecutionContext
) -> ExecutionContext | None:
    """Apply one or more assignment-only shell declarations."""
    if not arguments or any(not assignment(argument) for argument in arguments):
        return None
    for argument in arguments:
        context = assign(argument, context)
    return context


def at(context: ExecutionContext, cwd: str) -> ExecutionContext:
    owned = (
        git_directory_owned(cwd, context.git_dir)
        if context.git_dir is not None
        else repository_owned(cwd)
    )
    return replace(context, cwd=cwd, cwd_owned=owned)


def remote_url(
    context: ExecutionContext, remote: str, kind: str, value: str
) -> ExecutionContext:
    """Record a supported remote URL change for later shell segments."""
    remotes = {(name, key): current for name, key, current in context.remote_urls}
    remotes[(remote.casefold(), kind)] = value
    values = tuple((name, key, current) for (name, key), current in remotes.items())
    return replace(context, remote_urls=values)


def unset(context: ExecutionContext, key: str) -> ExecutionContext:
    git_dir = None if key == "GIT_DIR" else context.git_dir
    gh_repo = None if key == "GH_REPO" else context.gh_repo
    owned = repository_owned(context.cwd) if git_dir is None else context.cwd_owned
    environment = dict(context.environment)
    environment.pop(key, None)
    return replace(
        context,
        cwd_owned=owned,
        git_dir=git_dir,
        gh_repo=gh_repo,
        environment=tuple(environment.items()),
    )


def export_variables(
    arguments: list[str], context: ExecutionContext
) -> ExecutionContext | None:
    """Apply the supported stateful Bash export grammar, or reject ambiguity."""
    if arguments[:1] == ["--"]:
        arguments = arguments[1:]
    if not arguments or arguments == ["-p"]:
        return context
    if any(argument.startswith("-") for argument in arguments):
        return None
    for argument in arguments:
        if assignment(argument):
            context = assign(argument, context)
        elif VARIABLE_NAME.fullmatch(argument) is None:
            return None
        elif argument not in dict(context.environment):
            context = assign(f"{argument}=", context)
    return context


def printf_assignment(
    arguments: list[str], context: ExecutionContext
) -> ExecutionContext | None:
    """Apply the bounded ``printf -v NAME %s VALUE`` assignment grammar."""
    if (
        len(arguments) != 4
        or arguments[0] != "-v"
        or arguments[2] != "%s"
        or VARIABLE_NAME.fullmatch(arguments[1]) is None
    ):
        return None
    return assign(f"{arguments[1]}={arguments[3]}", context)


def unset_variables(
    arguments: list[str], context: ExecutionContext
) -> ExecutionContext | None:
    """Apply variable unsets while rejecting function and malformed forms."""
    if arguments[:1] == ["--"]:
        arguments = arguments[1:]
    elif arguments[:1] == ["-v"]:
        arguments = arguments[1:]
    if not arguments or any(
        VARIABLE_NAME.fullmatch(argument) is None for argument in arguments
    ):
        return None
    for argument in arguments:
        context = unset(context, argument)
    return context


def clear(context: ExecutionContext) -> ExecutionContext:
    return replace(
        context,
        cwd_owned=repository_owned(context.cwd),
        git_dir=None,
        gh_repo=None,
        environment=(),
        opaque_environment=False,
    )


def expand_tokens(tokens: list[str], context: ExecutionContext) -> list[str] | None:
    if context.opaque_environment:
        return None
    expanded = [expand(token, context) for token in tokens]
    if any(token is None for token in expanded):
        return None
    resolved = [token for token in expanded if token is not None]
    if tokens and VARIABLE_REFERENCE.fullmatch(tokens[0]):
        return resolved[0].split() + resolved[1:]
    return resolved


def expand(value: str, context: ExecutionContext) -> str | None:
    environment = dict(context.environment)
    missing = False

    def replace(match: re.Match[str]) -> str:
        nonlocal missing
        key = match.group("braced") or match.group("plain")
        if key not in environment:
            missing = True
            return ""
        return environment[key]

    expanded = VARIABLE_REFERENCE.sub(replace, value)
    return (
        None
        if missing or "$" in expanded
        else expanded.replace(SINGLE_QUOTED_DOLLAR, "$")
    )


def git_config(context: ExecutionContext) -> dict[str, str] | None:
    """Return one complete indexed Git configuration environment, or reject ambiguity."""
    relevant = {
        key: value
        for key, value in context.environment
        if key.startswith("GIT_CONFIG_")
    }
    if not relevant:
        return {}
    count_text = relevant.pop("GIT_CONFIG_COUNT", None)
    if count_text is None or not count_text.isascii() or not count_text.isdigit():
        return None
    count = int(count_text)
    if count > 64:
        return None
    entries: dict[str, str] = {}
    for index in range(count):
        key = relevant.pop(f"GIT_CONFIG_KEY_{index}", None)
        value = relevant.pop(f"GIT_CONFIG_VALUE_{index}", None)
        if (
            key is None
            or value is None
            or not key
            or any(char in key + value for char in "\0\r\n")
            or key in entries
        ):
            return None
        entries[key] = value
    return None if relevant else entries
