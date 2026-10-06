"""Credential exposure checks for the shared shell evaluator."""

from __future__ import annotations

from collections.abc import Sequence

from .execution_context import assignment
from .execution_context_types import CommandEffect, ExecutionContext
from .invocation import Invocation


class CredentialPolicy:
    """Detect credential operations through the ordinary stateful effect walk."""

    detect_leading_credentials: bool = True
    redirection_executables: frozenset[str] = frozenset()

    @staticmethod
    def owns_opaque(command: str, context: ExecutionContext) -> bool:
        _ = (command, context)
        return False

    @staticmethod
    def opaque_invocation(invocation: Invocation) -> bool:
        _ = invocation
        return False

    @staticmethod
    def command(
        invocation: Invocation, outer: ExecutionContext, depth: int
    ) -> tuple[bool, CommandEffect] | None:
        _ = depth
        if _credential_environment(invocation.context):
            return True, CommandEffect(None)
        if invocation.executable != "gh":
            return None
        return (
            invocation.arguments[:2] == ["auth", "token"]
            or _auth_status_exposes_token(invocation.arguments)
            or _credential_header(invocation.arguments),
            CommandEffect(outer, outer),
        )


def credential_assignment(tokens: Sequence[str]) -> bool:
    """Recognize nonempty GitHub token assignments before an executable."""
    return any(
        assignment(token)
        and token.split("=", 1)[0]
        in {
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "GH_ENTERPRISE_TOKEN",
            "GITHUB_ENTERPRISE_TOKEN",
        }
        and bool(token.split("=", 1)[1])
        for token in tokens
    )


def _credential_environment(context: ExecutionContext) -> bool:
    return credential_assignment(
        tuple(f"{key}={value}" for key, value in context.environment)
    )


def _credential_header(arguments: list[str]) -> bool:
    for index, argument in enumerate(arguments):
        header = (
            arguments[index + 1]
            if argument in {"-H", "--header"} and index + 1 < len(arguments)
            else argument.split("=", 1)[1]
            if argument.startswith(("-H=", "--header="))
            else None
        )
        if header is None:
            continue
        name, separator, value = header.partition(":")
        if (
            separator
            and name.casefold() in {"authorization", "x-github-token"}
            and value.strip()
        ):
            return True
    return False


def _auth_status_exposes_token(arguments: list[str]) -> bool:
    return arguments[:2] == ["auth", "status"] and any(
        option in {"-t", "--show-token", "--with-token"} for option in arguments[2:]
    )
