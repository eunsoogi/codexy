"""Read-only, fail-closed repository identity checks."""

from __future__ import annotations

import os
import re
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path

from .repository_aliases import collect as collect_aliases


def read_text_file(path: Path) -> str | None:
    """Read one bounded regular file without following a symlink."""
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
    """Return the nearest ordinary Git worktree root, if one is present."""
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
        marker = read_text_file(dot_git)
        if marker is None or len(marker.splitlines()) != 1 or not marker.startswith("gitdir: "):
            return None
        return root
    return None


@dataclass(frozen=True)
class UrlRewrite:
    prefix: str
    replacement: str
    push_only: bool = False


def repository_status(cwd: str) -> bool:
    """Return whether the command starts inside a discoverable Git worktree."""
    return worktree_root(Path(cwd)) is not None


def repository_owned(cwd: str) -> bool | None:
    if worktree_root(Path(cwd)) is None:
        return False
    return True if _find_config(Path(cwd)) is not None else None


def repository_owned_with_rewrites(
    cwd: str,
    git_dir: str | None,
    rewrites: list[UrlRewrite],
    push: bool,
    remote_urls: tuple[tuple[str, str, str], ...] = (),
) -> bool | None:
    """Keep Git destructive protection independent of remote or repository policy."""
    del rewrites, push, remote_urls
    return git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)


def git_directory_owned(cwd: str, target: str) -> bool | None:
    path = Path(target)
    if not path.is_absolute():
        path = Path(cwd) / path
    return True if read_text_file(path / "config") is not None else None


def git_aliases(cwd: str, git_dir: str | None = None) -> dict[str, str] | None:
    """Return Git's effective aliases across active configuration scopes."""
    return collect_aliases(cwd, git_dir, _git_config)


def git_url_rewrites(cwd: str, git_dir: str | None = None) -> list[UrlRewrite] | None:
    """Return URL rewrites across every active Git configuration scope."""
    command = ["git", "-C", cwd]
    if git_dir is not None:
        command.append(f"--git-dir={git_dir}")
    command.extend(
        [
            "config",
            "--includes",
            "--null",
            "--get-regexp",
            r"^url\..*\.(insteadof|pushinsteadof)$",
        ]
    )
    try:
        result = subprocess.run(command, capture_output=True, check=False, timeout=1)
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode not in {0, 1} or len(result.stdout) > 65536:
        return None
    rewrites: list[UrlRewrite] = []
    try:
        for record in (item for item in result.stdout.split(b"\0") if item):
            variable, separator, value = record.partition(b"\n")
            key, prefix = (
                variable.decode("utf-8", "strict"),
                value.decode("utf-8", "strict"),
            )
            match = re.fullmatch(
                r"url\.(.+)\.(insteadof|pushinsteadof)", key, re.IGNORECASE
            )
            if (
                not separator
                or match is None
                or not prefix
                or any(char in key + prefix for char in "\0\r\n")
            ):
                return None
            rewrites.append(
                UrlRewrite(
                    prefix, match.group(1), match.group(2).casefold() == "pushinsteadof"
                )
            )
    except UnicodeError:
        return None
    return rewrites


def _git_config(cwd: str, git_dir: str | None) -> str | None:
    if git_dir is None:
        return _find_config(Path(cwd))
    path = Path(git_dir)
    return read_text_file((path if path.is_absolute() else Path(cwd) / path) / "config")


def _find_config(cwd: Path) -> str | None:
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
            return read_text_file(dot_git / "config")
        marker = read_text_file(dot_git)
        if (
            marker is None
            or len(marker.splitlines()) != 1
            or not marker.startswith("gitdir: ")
        ):
            return None
        gitdir = Path(marker.splitlines()[0][8:])
        if not gitdir.is_absolute():
            gitdir = dot_git.parent / gitdir
        common = read_text_file(gitdir.resolve() / "commondir")
        target = (
            gitdir.resolve()
            if common is None
            else (gitdir.resolve() / common.strip()).resolve()
        )
        return read_text_file(target / "config")
    return None
