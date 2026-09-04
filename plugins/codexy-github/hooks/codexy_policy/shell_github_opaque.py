"""Opaque-shell selector owned by repository-GitHub-command admission."""

from .execution_context import ExecutionContext
from .github_workflow import read_only
from .invocation import Invocation
from .shell_opaque import (
    contains_policy_executable,
    unresolved_invocation,
    unresolved_protected_effect,
)


def owns(command: str, context: ExecutionContext) -> bool:
    return unresolved_protected_effect(command, context) or contains_policy_executable(
        command, context, "gh"
    )


def owns_invocation(invocation: Invocation) -> bool:
    """Classify an already parsed opaque invocation without reparsing its data."""
    return (
        invocation.executable == "gh" and not _opaque_read_only(invocation.arguments)
    ) or unresolved_invocation(invocation)


def _opaque_read_only(arguments: list[str]) -> bool:
    """Allow only read forms whose opaque data cannot alter the effect class."""
    if not read_only(arguments):
        return False
    if arguments[:1] == ["api"]:
        return not any("$" in argument for argument in arguments[1:])
    if arguments[:2] == ["auth", "status"]:
        return not any("$" in argument for argument in arguments[2:])
    return True
