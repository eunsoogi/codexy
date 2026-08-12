"""Opaque-shell selector owned by repository-GitHub-command admission."""

import re


def owns(command: str) -> bool:
    return re.search(
        r"(?:^|[;&|()\s])gh(?=$|[;&|()\s])|\bGH_REPO\s*=", command
    ) is not None
