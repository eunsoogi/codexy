"""Opaque-shell selector owned by repository-GitHub-command admission."""

import re

from .execution_context import ExecutionContext
from .shell_opaque import contains_policy_executable

def owns(command: str, context: ExecutionContext) -> bool:
    return re.search(
        r"\bGH_REPO\s*=", command
    ) is not None or contains_policy_executable(command, context, "gh")
