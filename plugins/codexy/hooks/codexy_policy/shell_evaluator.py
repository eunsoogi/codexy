"""Evaluate shell commands after the public policy entrypoint builds context."""

from __future__ import annotations

import re
import shlex
from pathlib import Path

from .execution_context import (
    DYNAMIC_VALUE, SINGLE_QUOTED_DOLLAR, CommandEffect, ExecutionContext,
    after_external_command, at as context_at, git_config, remote_url,
)
from .git_command import normalize as normalize_git
from .git_options import normalize as normalize_git_options
from .github import forbidden as gh_forbidden
from .github_alias import expand as expand_gh_alias
from .invocation import resolve
from .repository import OWNED, UrlRewrite, github_identity, identity, rewrite_url
from .shell_builtins import hash_path_alias, rm_forbidden
from .shell_context import changed_directory, flag
from .shell_groups import GroupSyntaxError, parse
from .shell_opaque import dynamic_control_executable, owns_opaque, separate_lines
from .shell_sequence import evaluate as evaluate_sequence

OPAQUE = re.compile(r"\$\(|`|<<<?|\b(?:eval|if|for|while|until|case)\b")
SUBCOMMAND = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")
CONTROL = re.compile(r"<<<?|\b(?:if|for|while|until|case)\b")
REMOTE_URL_CONFIG = re.compile(r"remote\.([A-Za-z0-9._-]+)\.(url|pushurl)", re.IGNORECASE)


def forbidden(command: str, context: ExecutionContext, depth: int, mode: str) -> bool:
    if depth > 3 or SINGLE_QUOTED_DOLLAR in command:
        return True
    lexical_command = command
    if OPAQUE.search(command):
        if context.cwd_owned is not False and owns_opaque(command, mode):
            return True
        try:
            opaque_tokens = shlex.split(command)
            if opaque_tokens and opaque_tokens[0].rsplit("/", 1)[-1].lower() == "eval":
                evaluated = opaque_tokens[1:]
                if evaluated[:1] == ["--"]:
                    evaluated = evaluated[1:]
                if forbidden(" ".join(evaluated), context, depth + 1, mode):
                    return True
            elif _explicit_owned(opaque_tokens) is True:
                return True
        except ValueError:
            return True
        for match in SUBCOMMAND.finditer(command):
            nested = match.group(1) if match.group(1) is not None else match.group(2)
            if forbidden(nested, context, depth + 1, mode):
                return True
        lexical_command = SUBCOMMAND.sub(DYNAMIC_VALUE, command)
        if CONTROL.search(command):
            return owns_opaque(command, mode) or dynamic_control_executable(command)
    try:
        lexer = shlex.shlex(separate_lines(lexical_command), posix=True, punctuation_chars=";&|(){}")
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return context.cwd_owned is not False
    try:
        sequence = parse(tokens)
    except GroupSyntaxError:
        return True
    return evaluate_sequence(
        sequence, context, depth,
        lambda tokens, current, current_depth: _segment(
            tokens, current, current_depth, mode
        ),
    )[0]


def _segment(
    tokens: list[str], context: ExecutionContext, depth: int, mode: str,
) -> tuple[bool, CommandEffect]:
    invocation = resolve(tokens, context, depth)
    if invocation is None:
        return True, CommandEffect(None)
    if invocation.script is not None:
        return not invocation.script or forbidden(
            invocation.script, invocation.context, depth + 1, mode
        ), CommandEffect(context)
    if invocation.opaque:
        return mode in {"all", "destructive"} or owns_opaque(
            " ".join(tokens), mode
        ), CommandEffect(None)
    if invocation.executable is None:
        return False, CommandEffect(invocation.context)
    if invocation.executable == "false":
        return False, CommandEffect(None, context)
    if invocation.executable == "true":
        return False, CommandEffect(context)
    if invocation.executable == "test":
        return False, _test_effect(invocation.arguments, context)
    if invocation.executable in {"cd", "pushd", "popd"}:
        directory = changed_directory(
            [invocation.executable, *invocation.arguments], invocation.context.cwd
        )
        return (True, CommandEffect(None)) if directory.opaque else (
            False, CommandEffect(context_at(invocation.context, directory.cwd))
        )
    if invocation.executable in {".", "source"}:
        return True, CommandEffect(None)
    if invocation.executable == "hash" and hash_path_alias(invocation.arguments):
        return True, CommandEffect(None)
    if invocation.executable == "git":
        denied, remote = _git(invocation.arguments, invocation.context, depth, mode)
        denied = denied if mode != "github" else False
        if remote is None:
            return denied, CommandEffect(context)
        if invocation.context.cwd != context.cwd or invocation.context.git_dir != context.git_dir:
            return True, CommandEffect(None)
        return denied, CommandEffect(remote_url(context, *remote))
    if invocation.executable == "gh":
        if mode == "destructive":
            return False, CommandEffect(context)
        gh_owned = github_identity(invocation.context.gh_repo) == OWNED if invocation.context.gh_repo is not None else None
        arguments = expand_gh_alias(invocation.arguments)
        return arguments is None or gh_forbidden(arguments, invocation.context.cwd, invocation.context.cwd_owned, gh_owned), CommandEffect(context)
    if invocation.executable == "rm":
        denied = invocation.context.cwd_owned is not False and rm_forbidden(invocation.arguments)
        return denied if mode != "github" else False, CommandEffect(context)
    effect = after_external_command(
        invocation.executable, invocation.arguments, context,
    )
    return (True, CommandEffect(None)) if effect is None else (False, effect)


def _test_effect(arguments: list[str], context: ExecutionContext) -> CommandEffect:
    """Model only deterministic ``test -e`` outcomes; preserve both branches otherwise."""
    if len(arguments) != 2 or arguments[0] != "-e":
        return CommandEffect(context, context)
    path = Path(arguments[1])
    candidate = path if path.is_absolute() else Path(context.cwd) / path
    return CommandEffect(context) if candidate.exists() else CommandEffect(None, context)


def _git(
    args: list[str], context: ExecutionContext, depth: int, mode: str
) -> tuple[bool, tuple[str, str, str] | None]:
    environment_config = git_config(context)
    if environment_config is None:
        return True, None
    invocation = normalize_git(args, context.cwd, context.cwd_owned, context.git_dir, _config_owned, environment_config, context.remote_urls)
    if invocation is None:
        return True, None
    if invocation.alias_command is not None:
        alias_context = ExecutionContext(
            invocation.cwd,
            invocation.cwd_owned,
            invocation.git_dir,
            context.gh_repo,
            context.environment,
            context.opaque_environment,
            context.remote_urls,
            context.opaque_repository_state,
            context.executable_aliases,
        )
        return not invocation.alias_command or forbidden(
            invocation.alias_command, alias_context, depth + 1, mode
        ), None
    if invocation.operation is None:
        return False, None
    if invocation.operation == "config":
        remote = REMOTE_URL_CONFIG.fullmatch(invocation.arguments[0]) if invocation.arguments else None
        if remote is not None and len(invocation.arguments) == 2 and invocation.arguments[1] and not any(
            char in invocation.arguments[1] for char in "\0\r\n"
        ):
            return False, (remote.group(1), remote.group(2).casefold(), invocation.arguments[1])
        if any(REMOTE_URL_CONFIG.fullmatch(argument) for argument in invocation.arguments):
            return True, None
    if invocation.operation == "remote" and invocation.arguments[:1] in (["add"], ["set-url"]):
        if len(invocation.arguments) != 3 or not invocation.arguments[1] or any(char in invocation.arguments[1] + invocation.arguments[2] for char in "\0\r\n"):
            return True, None
        return False, (invocation.arguments[1], "url", invocation.arguments[2])
    push_like = invocation.operation in {"push", "send-pack"}
    target_owned = _explicit_owned(
        invocation.arguments, list(invocation.rewrites), push_like
    )
    applies = target_owned is True or (
        target_owned is None
        and (context.opaque_repository_state or invocation.cwd_owned is not False)
    )
    arguments = normalize_git_options(invocation.operation, invocation.arguments)
    if arguments is None:
        return applies, None
    if push_like:
        forced = any(arg in {"--force", "--force-with-lease", "--mirror"} or arg.startswith(("--force=", "--force-with-lease=", "--mirror=")) or (arg.startswith("-") and not arg.startswith("--") and "f" in arg[1:]) or arg.startswith("+") for arg in arguments)
        return applies and forced, None
    return applies and ((invocation.operation == "reset" and "--hard" in arguments) or (invocation.operation == "clean" and flag(arguments, "f", "--force"))), None


def _explicit_owned(
    args: list[str], rewrites: list[UrlRewrite] | None = None, push: bool = False,
) -> bool | None:
    rewritten = [identity(rewrite_url(arg, rewrites or [], push)) for arg in args]
    identities = [item for item in rewritten if item is not None]
    return None if not identities else OWNED in identities


def _config_owned(config: str) -> bool:
    return "=" in config and identity(config.split("=", 1)[1]) == OWNED
