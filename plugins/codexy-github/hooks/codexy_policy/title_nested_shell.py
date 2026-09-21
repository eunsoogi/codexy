"""Reuse shell title checks for literal tools.exec_command calls."""

from __future__ import annotations

from pathlib import Path

from .title_nested_fields import object_fields
from .title_nested_parser import Token, arguments, matching
from .title_shell import forbidden as shell_forbidden


def forbidden(tokens: list[Token], index: int, cwd: object) -> bool:
    prefix = tokens[index : index + 4]
    if [(token.kind, token.value) for token in prefix] != [
        ("identifier", "tools"),
        ("punctuation", "."),
        ("identifier", "exec_command"),
        ("punctuation", "("),
    ]:
        return False
    close = matching(tokens, index + 3, "(", ")")
    args = arguments(tokens, index + 4, close)
    if len(args) != 1 or not args[0]:
        return False
    fields = object_fields(args[0])
    if fields is None or not isinstance(fields.get("cmd"), str):
        return False
    workdir = fields.get("workdir", cwd)
    if workdir is None:
        workdir = cwd
    if isinstance(workdir, str) and not Path(workdir).is_absolute():
        workdir = str(Path(cwd) / workdir) if isinstance(cwd, str) else None
    return shell_forbidden(fields["cmd"], workdir)
