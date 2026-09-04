"""Stable, sanitized rule and remediation details for policy denials."""

from __future__ import annotations

import re
from typing import Protocol

from .execution_context_types import CommandEffect, ExecutionContext
from .invocation import Invocation
from .shell_opaque import resolved_segments

SAFE_TOKEN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")


class DiagnosticPolicy(Protocol):
    def opaque_invocation(self, invocation: Invocation) -> bool: ...

    def command(
        self, invocation: Invocation, outer: ExecutionContext, depth: int
    ) -> tuple[bool, CommandEffect] | None: ...


def describe(
    code: str,
    command: str | None = None,
    context: ExecutionContext | None = None,
    policy: DiagnosticPolicy | None = None,
) -> str:
    rule, operation, remediation = _details(code, command, context, policy)
    return f"{code}: rule={rule}; operation={operation}; remediation={remediation}"


def _details(
    code: str,
    command: str | None,
    context: ExecutionContext | None,
    policy: DiagnosticPolicy | None,
) -> tuple[str, str, str]:
    if code == "CREDENTIAL_EXPOSURE":
        return (
            "credential-exposure",
            "GitHub credential access",
            "remove token/header credentials and use the host-authenticated session",
        )
    if code == "UNRESOLVED_TARGET":
        return (
            "request-shape",
            "repository GitHub command",
            "provide a string command and a repository working directory",
        )
    if code == "UNRESOLVED_PROTECTED_EFFECT":
        return (
            "unresolved-protected-effect",
            "dynamic or opaque protected command",
            "spell out the executable, repository, and arguments in the supported grammar",
        )
    invocation = _invocation(command, context, policy)
    if code == "REMOTE_MUTATION":
        return _remote(invocation)
    if code == "DESTRUCTIVE_EFFECT":
        return _destructive(invocation)
    return (
        "runtime-boundary",
        "protected operation",
        "use a fixed, explicitly scoped command",
    )


def _invocation(
    command: str | None,
    context: ExecutionContext | None,
    policy: DiagnosticPolicy | None,
) -> Invocation | None:
    if command is None or context is None:
        return None
    try:
        walked = resolved_segments(command, context)
    except (OSError, TypeError, ValueError):
        return None
    if walked is None:
        return None
    for segment in walked:
        invocation = segment.invocation
        if (
            invocation is not None
            and invocation.executable in {"gh", "git", "rm", "find"}
            and policy is not None
            and _denied_invocation(invocation, policy)
        ):
            return invocation
    return next(
        (
            segment.invocation
            for segment in walked
            if segment.invocation is not None
            and segment.invocation.executable in {"gh", "git", "rm", "find"}
        ),
        None,
    )


def _denied_invocation(invocation: Invocation, policy: DiagnosticPolicy) -> bool:
    if invocation.opaque and policy.opaque_invocation(invocation):
        return True
    if invocation.script is not None:
        return False
    result = policy.command(invocation, invocation.context, 0)
    return result is not None and result[0]


def _remote(invocation: Invocation | None) -> tuple[str, str, str]:
    if invocation is not None and invocation.executable == "gh":
        args = invocation.arguments
        if args[:2] == ["workflow", "run"]:
            return (
                "workflow-dispatch",
                "governed GitHub workflow dispatch",
                "use plugin-version-bump.yml with the owned repository and version/issue fields",
            )
        if args[:2] == ["run", "rerun"]:
            return (
                "workflow-rerun",
                "governed GitHub workflow retry",
                "use one positive numeric run id with the owned repository",
            )
        return (
            "github-mutation",
            _safe_gh_operation(args),
            "use the typed GitHub route with the owned repository and approved payload",
        )
    if _git_operation(invocation) == "push":
        return (
            "git-remote-update",
            "Git remote update",
            "use an explicit non-force push to a named branch; delete/prune/all/tags forms remain denied",
        )
    if _git_operation(invocation) == "add":
        return (
            "staging-scope",
            "local Git staging",
            "name files after --; do not use -A, -u, or .",
        )
    return (
        "github-mutation",
        "remote repository mutation",
        "use an explicitly owned repository and a supported typed operation",
    )


def _destructive(invocation: Invocation | None) -> tuple[str, str, str]:
    operation = _git_operation(invocation)
    if invocation is not None and invocation.executable == "find":
        return (
            "bounded-cache-cleanup",
            "recursive generated-cache cleanup",
            "limit -exec to rm -rf {} + for a named generated cache under the current repository",
        )
    if invocation is not None and invocation.executable == "rm":
        return (
            "bounded-local-deletion",
            "recursive local deletion",
            "name one generated cache directory under the current repository; reject roots, traversal, and dynamic targets",
        )
    if operation == "add":
        return (
            "staging-scope",
            "local Git staging",
            "name files after --; do not use -A, -u, or .",
        )
    if operation == "push":
        return (
            "git-remote-update",
            "Git remote update",
            "keep remote updates explicit and non-force; delete/prune/all/tags forms remain denied",
        )
    return (
        "destructive-effect",
        "destructive local operation",
        "use a fixed command with an explicitly bounded target",
    )


def _git_operation(invocation: Invocation | None) -> str | None:
    if invocation is None or invocation.executable != "git":
        return None
    return next(
        (
            argument
            for argument in invocation.arguments
            if argument in {"add", "push", "send-pack", "reset", "clean"}
        ),
        None,
    )


def _safe_gh_operation(args: list[str]) -> str:
    words = [word for word in args[:2] if SAFE_TOKEN.fullmatch(word)]
    return "gh " + " ".join(words) if words else "GitHub CLI mutation"
