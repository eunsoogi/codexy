"""Repository-relative safe text reads."""

from pathlib import Path

from .repository_policy import read_text_file


def read_text(cwd: str, target: str) -> str | None:
    path = Path(target)
    return (
        None
        if target == "-"
        else read_text_file(path if path.is_absolute() else Path(cwd) / path)
    )
