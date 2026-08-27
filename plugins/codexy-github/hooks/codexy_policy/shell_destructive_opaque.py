"""Opaque-shell selector owned by destructive-command admission."""

from .execution_context import ExecutionContext
from .invocation import Invocation
from .shell_opaque import (
    contains_policy_executable,
    unresolved_alias_transition,
    unresolved_protected_effect,
    unresolved_invocation,
)


def owns(command: str, context: ExecutionContext) -> bool:
    return (
        unresolved_protected_effect(command, context)
        or unresolved_alias_transition(command, context)
        or contains_policy_executable(command, context, "git")
    )


def owns_invocation(invocation: Invocation) -> bool:
    """Classify an already parsed opaque invocation without reparsing its data."""
    if invocation.executable != "git":
        return unresolved_invocation(invocation)
    operation = next((arg for arg in invocation.arguments if not arg.startswith("-")), None)
    return operation not in {"show", "status", "log", "diff", "rev-parse", "worktree", "branch", "ls-remote", "check-ref-format", "fetch"}
