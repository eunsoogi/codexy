"""Read-only, fail-closed repository identity checks."""

from __future__ import annotations

import configparser
import os
import re
import stat
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

OWNED = ("github.com", "eunsoogi", "codexy")
REMOTE = re.compile(r'^remote "[^"\r\n]+"$')
SCP = re.compile(r"^(?:[A-Za-z0-9._-]+@)?(?P<host>[A-Za-z0-9.-]+):(?P<path>[^\s?#]+)$")


@dataclass(frozen=True)
class UrlRewrite:
    prefix: str
    replacement: str
    push_only: bool = False


def _text(path: Path) -> str | None:
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


def identity(url: str) -> tuple[str, str, str] | None:
    match = None if "://" in url else SCP.fullmatch(url)
    if match:
        host, path = match.group("host"), match.group("path")
    else:
        parsed = urlsplit(url)
        if parsed.scheme not in {"http", "https", "ssh", "git"} or not parsed.hostname or parsed.password or parsed.query or parsed.fragment:
            return None
        host, path = parsed.hostname, parsed.path.lstrip("/")
    parts = path.removesuffix(".git").split("/")
    return (host.lower(), parts[0].lower(), parts[1].lower()) if len(parts) == 2 and all(parts) else None


def github_identity(value: str) -> tuple[str, str, str] | None:
    if "://" not in value:
        value = "https://" + value if value.count("/") == 2 else "https://github.com/" + value
    return identity(value)


def repository_owned(cwd: str) -> bool | None:
    return _config_owned(_find_config(Path(cwd)))


def repository_owned_with_rewrites(
    cwd: str, git_dir: str | None, rewrites: list[UrlRewrite], push: bool,
) -> bool | None:
    """Classify repository remotes after command-scoped Git URL rewriting."""
    config = _git_config(cwd, git_dir)
    return _config_owned(config, rewrites, push)


def git_directory_owned(cwd: str, target: str) -> bool | None:
    path = Path(target)
    if not path.is_absolute():
        path = Path(cwd) / path
    return _config_owned(_text(path / "config"))


def git_aliases(cwd: str, git_dir: str | None = None) -> dict[str, str] | None:
    """Return repository aliases only when their config is a safe regular file."""
    config = _git_config(cwd, git_dir)
    if config is None:
        return None
    try:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.read_string(config)
    except configparser.Error:
        return None
    sections = [section for section in parser.sections() if section.casefold() == "alias"]
    if not sections:
        return {}
    aliases = {key.casefold(): value for section in sections for key, value in parser[section].items()}
    return aliases if all(key and "=" not in key and "\n" not in value and "\r" not in value for key, value in aliases.items()) else None


def read_text(cwd: str, target: str) -> str | None:
    path = Path(target)
    return None if target == "-" else _text(path if path.is_absolute() else Path(cwd) / path)


def _config_owned(
    config: str | None, inline_rewrites: list[UrlRewrite] | None = None, push: bool = False,
) -> bool | None:
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
                    for value in (url, pushurl) if value
                )
    except configparser.Error:
        return None
    if not identities or any(item is None for item in identities):
        return None
    return OWNED in identities


def rewrite_url(value: str, rewrites: list[UrlRewrite], push: bool) -> str:
    matches = [item for item in rewrites if (push or not item.push_only) and value.startswith(item.prefix)]
    if not matches:
        return value
    selected = max(matches, key=lambda item: len(item.prefix))
    return selected.replacement + value[len(selected.prefix):]


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
    return _text((path if path.is_absolute() else Path(cwd) / path) / "config")


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
            return _text(dot_git / "config")
        marker = _text(dot_git)
        if marker is None or len(marker.splitlines()) != 1 or not marker.startswith("gitdir: "):
            return None
        gitdir = Path(marker.splitlines()[0][8:])
        if not gitdir.is_absolute():
            gitdir = dot_git.parent / gitdir
        common = _text(gitdir.resolve() / "commondir")
        target = gitdir.resolve() if common is None else (gitdir.resolve() / common.strip()).resolve()
        return _text(target / "config")
    return None
