"""Opaque-shell selector owned by destructive-command admission."""

import re


def owns(command: str) -> bool:
    return re.search(
        r"(?:^|[;&|()\s])(?:git|cd|source|\.|rm|pushd|popd)(?=$|[;&|()\s])"
        r"|\b(?:GIT_DIR|GIT_COMMON_DIR)\s*=",
        command,
    ) is not None
