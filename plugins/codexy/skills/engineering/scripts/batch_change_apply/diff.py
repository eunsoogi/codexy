"""Readable diffs for one selected output file."""

from __future__ import annotations

import difflib


def unified(old: bytes, new: bytes, path: str) -> str:
    """Return a text diff, or a concise binary-difference marker."""
    try:
        old_text = old.decode("utf-8")
        new_text = new.decode("utf-8")
    except UnicodeDecodeError:
        return f"Binary files a/{path} and b/{path} differ\n"
    if "\x00" in old_text or "\x00" in new_text:
        return f"Binary files a/{path} and b/{path} differ\n"
    return "".join(
        difflib.unified_diff(
            old_text.splitlines(keepends=True),
            new_text.splitlines(keepends=True),
            fromfile=f"a/{path}" if old else "/dev/null",
            tofile=f"b/{path}",
        )
    )
