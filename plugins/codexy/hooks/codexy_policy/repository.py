"""Read-only, fail-closed repository identity checks."""

from __future__ import annotations

import configparser
import os
import re
import stat
from pathlib import Path
from urllib.parse import urlsplit

OWNED = ("github.com", "eunsoogi", "codexy")
REMOTE = re.compile(r'^remote "[^"\r\n]+"$')
SCP = re.compile(r"^(?:[A-Za-z0-9._-]+@)?(?P<host>[A-Za-z0-9.-]+):(?P<path>[^\s?#]+)$")


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
    config = _find_config(Path(cwd))
    return _config_owned(config)


def git_directory_owned(cwd: str, target: str) -> bool | None:
    path = Path(target)
    if not path.is_absolute():
        path = Path(cwd) / path
    return _config_owned(_text(path / "config"))


def git_aliases(cwd: str, git_dir: str | None = None) -> dict[str, str] | None:
    """Return repository aliases only when their config is a safe regular file."""
    if git_dir is None:
        config = _find_config(Path(cwd))
    else:
        path = Path(git_dir)
        config = _text((path if path.is_absolute() else Path(cwd) / path) / "config")
    if config is None:
        return None
    try:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.optionxform = str
        parser.read_string(config)
    except configparser.Error:
        return None
    if not parser.has_section("alias"):
        return {}
    aliases = dict(parser["alias"])
    return aliases if all(key and "=" not in key and "\n" not in value and "\r" not in value for key, value in aliases.items()) else None


def read_text(cwd: str, target: str) -> str | None:
    path = Path(target)
    return None if target == "-" else _text(path if path.is_absolute() else Path(cwd) / path)


def _config_owned(config: str | None) -> bool | None:
    if config is None:
        return None
    try:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.read_string(config)
        rewrites = [
            (parser[name].get("insteadOf", ""), name[5:-1])
            for name in parser.sections()
            if name.startswith('url "') and name.endswith('"') and parser[name].get("insteadOf")
        ]
        identities = [
            identity(_rewrite(parser[name].get(key, ""), rewrites))
            for name in parser.sections()
            if REMOTE.fullmatch(name)
            for key in ("url", "pushurl")
            if parser[name].get(key)
        ]
    except configparser.Error:
        return None
    if not identities or any(item is None for item in identities):
        return None
    return OWNED in identities


def _rewrite(value: str, rewrites: list[tuple[str, str]]) -> str:
    matches = [(prefix, replacement) for prefix, replacement in rewrites if value.startswith(prefix)]
    if not matches:
        return value
    prefix, replacement = max(matches, key=lambda item: len(item[0]))
    return replacement + value[len(prefix):]


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
