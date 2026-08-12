"""Opaque-shell selector owned by destructive-command admission."""

import re

from .execution_context import ExecutionContext
from .shell_opaque import contains_policy_executable

def owns(command: str, context: ExecutionContext) -> bool:
    return re.search(
        r"(?:^|[;&|()\s])(?:cd|source|\.|rm|pushd|popd)(?=$|[;&|()\s])"
        r"|\b(?:GIT_DIR|GIT_COMMON_DIR)\s*=",
        command,
    ) is not None or contains_policy_executable(command, context, "git")
