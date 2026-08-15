"""Small shared workflow mapping queries used by legacy and shard contracts."""

from __future__ import annotations

from collections.abc import Callable


def job_values(
    lines: list[str],
    key: str,
    mapping: Callable[[str], tuple[str, str] | None],
    scalar: Callable[[str], str],
) -> list[str]:
    values = []
    for line in lines:
        if (
            len(line) - len(line.lstrip(" ")) == 4
            and (entry := mapping(line)) is not None
            and entry[0] == key
        ):
            values.append(scalar(entry[1]))
    return values
