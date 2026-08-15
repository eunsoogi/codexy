"""Opaque-shell selector owned by repository-GitHub-command admission."""

from .execution_context import ExecutionContext
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
    return invocation.executable == "gh" or unresolved_invocation(invocation)
