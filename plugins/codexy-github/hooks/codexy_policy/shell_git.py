"""Concern-neutral structural Git parsing shared by shell policies."""

from __future__ import annotations

import re
from dataclasses import dataclass, replace

from .execution_context import ExecutionContext, git_config
from .git_command import normalize as normalize_git
from .git_options import normalize as normalize_git_options
from .repository import UrlRewrite, identity, rewrite_url
from .shell_context import flag

REMOTE_URL_CONFIG = re.compile(
    r"remote\.([A-Za-z0-9._-]+)\.(url|pushurl)", re.IGNORECASE
)


@dataclass(frozen=True)
class AliasCommand:
    command: str
    context: ExecutionContext


def evaluate(
    args: list[str], context: ExecutionContext
) -> tuple[bool, tuple[str, str, str] | None, AliasCommand | None]:
    environment_config = git_config(context)
    if environment_config is None:
        return True, None, None
    policy_status = context.policy_status
    if policy_status is None:
        return True, None, None
    owned_identity = context.policy_identity
    invocation = normalize_git(
        args,
        context.cwd,
        context.cwd_owned,
        context.git_dir,
        lambda config: _config_owned(config, owned_identity),
        environment_config,
        context.remote_urls,
    )
    if invocation is None:
        return True, None, None
    if invocation.alias_command is not None:
        normalized = replace(
            context,
            cwd=invocation.cwd,
            cwd_owned=invocation.cwd_owned,
            git_dir=invocation.git_dir,
        )
        return (
            not invocation.alias_command,
            None,
            AliasCommand(invocation.alias_command, normalized),
        )
    if invocation.operation is None:
        return False, None, None
    if invocation.operation == "config":
        remote = (
            REMOTE_URL_CONFIG.fullmatch(invocation.arguments[0])
            if invocation.arguments
            else None
        )
        if (
            remote is not None
            and len(invocation.arguments) == 2
            and invocation.arguments[1]
            and not any(char in invocation.arguments[1] for char in "\0\r\n")
        ):
            return (
                False,
                (remote.group(1), remote.group(2).casefold(), invocation.arguments[1]),
                None,
            )
        if any(
            REMOTE_URL_CONFIG.fullmatch(argument) for argument in invocation.arguments
        ):
            return True, None, None
    if invocation.operation == "remote" and invocation.arguments[:1] in (
        ["add"],
        ["set-url"],
    ):
        if (
            len(invocation.arguments) != 3
            or not invocation.arguments[1]
            or any(
                char in invocation.arguments[1] + invocation.arguments[2]
                for char in "\0\r\n"
            )
        ):
            return True, None, None
        return False, (invocation.arguments[1], "url", invocation.arguments[2]), None
    push_like = invocation.operation in {"push", "send-pack"}
    target_owned = explicit_owned(
        invocation.arguments, owned_identity, list(invocation.rewrites), push_like
    )
    applies = target_owned is True or (
        target_owned is None
        and (context.opaque_repository_state or invocation.cwd_owned is not False)
    )
    arguments = normalize_git_options(invocation.operation, invocation.arguments)
    if arguments is None:
        return applies, None, None
    if push_like:
        forced = any(
            arg in {"--force", "--force-with-lease", "--mirror"}
            or arg.startswith(("--force=", "--force-with-lease=", "--mirror="))
            or (arg.startswith("-") and not arg.startswith("--") and "f" in arg[1:])
            or arg.startswith("+")
            for arg in arguments
        )
        return applies and forced, None, None
    denied = (invocation.operation == "reset" and "--hard" in arguments) or (
        invocation.operation == "clean" and flag(arguments, "f", "--force")
    )
    return applies and denied, None, None


def explicit_owned(
    args: list[str],
    owned: tuple[str, str, str] | None,
    rewrites: list[UrlRewrite] | None = None,
    push: bool = False,
) -> bool | None:
    rewritten = [identity(rewrite_url(arg, rewrites or [], push)) for arg in args]
    identities = [item for item in rewritten if item is not None]
    return None if not identities or owned is None else owned in identities


def _config_owned(config: str, owned: tuple[str, str, str] | None) -> bool:
    return (
        owned is not None
        and "=" in config
        and identity(config.split("=", 1)[1]) == owned
    )
