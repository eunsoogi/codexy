"""Read-only, fail-closed repository identity checks."""

from __future__ import annotations

import configparser
import os
import re
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path

from .git_runtime_config import apply_remote_urls, remote_config

REMOTE = re.compile(r'^remote "[^"\r\n]+"$')
from .repository_aliases import collect as collect_aliases
from .repository_files import read_text
from .repository_identity import github_identity, identity
from .repository_policy import (
    policy_identity,
    policy_path_status,
    read_text_file,
    worktree_root,
)


@dataclass(frozen=True)
class UrlRewrite:
    prefix: str
    replacement: str
    push_only: bool = False


def repository_identity(cwd: str | None = None) -> tuple[str, str, str] | None:
    """Read the opt-in identity only from this worktree's root policy."""
    return policy_identity(cwd)


def repository_policy_status(cwd: str) -> bool | None:
    """Return valid, absent, or invalid for the single worktree-root policy."""
    root = worktree_root(Path(cwd))
    if root is None:
        return False
    policy = policy_path_status(root)
    if policy is False:
        return False
    if policy is None:
        return None
    owned = repository_identity(cwd)
    if owned is None:
        return None
    return True


def repository_owned(cwd: str) -> bool | None:
    if repository_policy_status(cwd) is None:
        return None
    owned = repository_identity(cwd)
    return False if owned is None else _config_owned(_find_config(Path(cwd)), owned)


def repository_owned_with_rewrites(
    cwd: str,
    git_dir: str | None,
    rewrites: list[UrlRewrite],
    push: bool,
    remote_urls: tuple[tuple[str, str, str], ...] = (),
) -> bool | None:
    """Classify repository remotes after command-scoped Git URL rewriting."""
    config = remote_config(cwd, git_dir, push, remote_urls)
    if repository_policy_status(cwd) is None:
        return None
    owned = repository_identity(cwd)
    if owned is None:
        return False
    if config:
        return _config_owned(config, owned, rewrites, push)
    return _config_owned(_git_config(cwd, git_dir), owned, rewrites, push, remote_urls)


def git_directory_owned(cwd: str, target: str) -> bool | None:
    path = Path(target)
    if not path.is_absolute():
        path = Path(cwd) / path
    owned = repository_identity(str(path.parent))
    return (
        False
        if owned is None
        else _config_owned(read_text_file(path / "config"), owned)
    )


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


def _config_owned(
    config: str | None,
    owned: tuple[str, str, str],
    inline_rewrites: list[UrlRewrite] | None = None,
    push: bool = False,
    remote_urls: tuple[tuple[str, str, str], ...] = (),
) -> bool | None:
    config = apply_remote_urls(config, remote_urls)
    if config is None:
        return None
    try:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.read_string(config)
        rewrites = _config_rewrites(parser) + (inline_rewrites or [])
        identities = []
        for name in parser.sections():
            if not REMOTE.fullmatch(name):
                continue
            url, pushurl = parser[name].get("url", ""), parser[name].get("pushurl", "")
            if push and pushurl:
                identities.append(identity(rewrite_url(pushurl, rewrites, False)))
            elif push and url:
                identities.append(identity(rewrite_url(url, rewrites, True)))
            else:
                identities.extend(
                    identity(rewrite_url(value, rewrites, False))
                    for value in (url, pushurl)
                    if value
                )
    except configparser.Error:
        return None
    if not identities or any(item is None for item in identities):
        return None
    return owned in identities


def rewrite_url(value: str, rewrites: list[UrlRewrite], push: bool) -> str:
    matches = [
        item
        for item in rewrites
        if (push or not item.push_only) and value.startswith(item.prefix)
    ]
    if not matches:
        return value
    selected = max(matches, key=lambda item: len(item.prefix))
    return selected.replacement + value[len(selected.prefix) :]


def _config_rewrites(parser: configparser.ConfigParser) -> list[UrlRewrite]:
    result: list[UrlRewrite] = []
    for section in parser.sections():
        match = re.fullmatch(r'url "([^"\r\n]+)"', section, re.IGNORECASE)
        if match is None:
            continue
        for key, push_only in (("insteadof", False), ("pushinsteadof", True)):
            prefix = parser[section].get(key, "")
            if prefix:
                result.append(UrlRewrite(prefix, match.group(1), push_only))
    return result


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
