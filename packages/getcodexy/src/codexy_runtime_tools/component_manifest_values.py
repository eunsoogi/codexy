"""JSON value validation shared by component manifest parsing."""

from __future__ import annotations

from typing import Any


def strings(value: Any, field: str, *, nonempty: bool = False) -> tuple[str, ...]:
    if (
        not isinstance(value, list)
        or (nonempty and not value)
        or any(not isinstance(item, str) or not item for item in value)
    ):
        raise ValueError(f"component manifest {field} must be strings")
    return tuple(value)


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"component manifest has duplicate key: {key}")
        result[key] = value
    return result
