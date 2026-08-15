"""Strict, project-local repository policy discovery."""

from __future__ import annotations

import json
import os
import re
import stat
from pathlib import Path

POLICY_PATH = (".codex", "repository-github-policy.json")
POLICY_SCHEMA = "codexy.repository-github-policy/v1"
REPOSITORY_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9-]*/[A-Za-z0-9._-]+$")


def read_text_file(path: Path) -> str | None:
    try:
        info = os.lstat(path)
        if stat.S_ISLNK(info.st_mode) or not stat.S_ISREG(info.st_mode):
            return None
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        try:
            data = os.read(descriptor, 65537)
        finally:
            os.close(descriptor)
        return data.decode("utf-8", "strict") if len(data) <= 65536 else None
    except (OSError, UnicodeError):
        return None


def worktree_root(cwd: Path) -> Path | None:
    if not cwd.is_absolute():
        return None
    for root in (cwd, *cwd.parents):
        dot_git = root / ".git"
        try:
            info = os.lstat(dot_git)
        except FileNotFoundError:
            continue
        except OSError:
            return None
        if stat.S_ISLNK(info.st_mode):
            return None
        if stat.S_ISDIR(info.st_mode):
            return root
        if stat.S_ISREG(info.st_mode) and read_text_file(dot_git) is not None:
            return root
        return None
    return None


def policy_path_status(root: Path) -> Path | None | bool:
    """Return the root policy, invalid, or absent without following links."""
    directory = root / POLICY_PATH[0]
    policy = directory / POLICY_PATH[1]
    try:
        directory_info = os.lstat(directory)
    except FileNotFoundError:
        return False
    except OSError:
        return None
    if stat.S_ISLNK(directory_info.st_mode) or not stat.S_ISDIR(directory_info.st_mode):
        return None
    try:
        policy_info = os.lstat(policy)
    except FileNotFoundError:
        return False
    except OSError:
        return None
    return policy if stat.S_ISREG(policy_info.st_mode) else None


def policy_identity(cwd: str | None = None) -> tuple[str, str, str] | None:
    root = worktree_root(Path.cwd() if cwd is None else Path(cwd))
    if root is None:
        return None
    policy = policy_path_status(root)
    if not isinstance(policy, Path):
        return None
    text = read_text_file(policy)
    if text is None:
        return None
    try:
        data = json.loads(text, object_pairs_hook=_unique_object)
    except (ValueError, json.JSONDecodeError):
        return None
    repository = (
        data.get("repository")
        if set(data) == {"schema", "repository"} and data.get("schema") == POLICY_SCHEMA
        else None
    )
    if not isinstance(repository, str) or REPOSITORY_NAME.fullmatch(repository) is None:
        return None
    owner, name = repository.split("/", 1)
    return "github.com", owner.casefold(), name.casefold()


def _unique_object(items: list[tuple[str, object]]) -> dict[str, object]:
    data: dict[str, object] = {}
    for key, value in items:
        if key in data:
            raise ValueError("duplicate policy key")
        data[key] = value
    return data
