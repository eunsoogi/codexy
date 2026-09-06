"""Protect Git reflog selectors while shell punctuation is tokenized."""

from __future__ import annotations

import re

_OPEN, _CLOSE = "\ue005", "\ue006"
_SELECTOR = re.compile(r"@\{([^\s{};&|()<>]*)\}")


def protect(command: str) -> str:
    return (
        _SELECTOR.sub(lambda match: f"@{_OPEN}{match.group(1)}{_CLOSE}", command)
        if "@{" in command
        else command
    )


def restore(tokens: list[str]) -> list[str]:
    return [token.replace(_OPEN, "{").replace(_CLOSE, "}") for token in tokens]
