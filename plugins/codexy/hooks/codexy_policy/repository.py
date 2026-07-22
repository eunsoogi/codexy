"""Read-only, fail-closed repository identity checks."""

from __future__ import annotations

import configparser
import os
import re
import stat
import subprocess
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

from .git_runtime_config import apply_remote_urls, remote_config

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
    remote_urls: tuple[tuple[str, str, str], ...] = (),
) -> bool | None:
    """Classify repository remotes after command-scoped Git URL rewriting."""
    config = remote_config(cwd, git_dir, push, remote_urls)
    if config:
        return _config_owned(config, rewrites, push)
    return _config_owned(_git_config(cwd, git_dir), rewrites, push, remote_urls)


def git_directory_owned(cwd: str, target: str) -> bool | None:
    path = Path(target)
    if not path.is_absolute():
        path = Path(cwd) / path
    return _config_owned(_text(path / "config"))


def git_aliases(cwd: str, git_dir: str | None = None) -> dict[str, str] | None:
    """Return Git's effective aliases across active configuration scopes."""
    command = ["git", "-C", cwd]
    if git_dir is not None:
        command.append(f"--git-dir={git_dir}")
    command.extend(["config", "--includes", "--null", "--get-regexp", r"^alias\."])
    try:
        result = subprocess.run(command, capture_output=True, check=False, timeout=1)
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode not in {0, 1} or len(result.stdout) > 65536:
        return None
    aliases: dict[str, str] = {}
    try:
        records = [record for record in result.stdout.split(b"\0") if record]
        for record in records:
            variable, separator, value = record.partition(b"\n")
            key = variable.decode("utf-8", "strict").casefold()
            command_text = value.decode("utf-8", "strict")
            if not separator or not key.startswith("alias."):
                return None
            alias = key.removeprefix("alias.")
            if not alias or "=" in alias or any(char in command_text for char in "\0\r\n"):
                return None
            aliases[alias] = command_text
    except UnicodeError:
        return None
    local = _aliases_from_config(_git_config(cwd, git_dir))
    if local is None:
        return None
    aliases.update(local)
    return aliases


def git_url_rewrites(cwd: str, git_dir: str | None = None) -> list[UrlRewrite] | None:
    """Return URL rewrites across every active Git configuration scope."""
    command = ["git", "-C", cwd]
    if git_dir is not None:
        command.append(f"--git-dir={git_dir}")
    command.extend(["config", "--includes", "--null", "--get-regexp", r"^url\..*\.(insteadof|pushinsteadof)$"])
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
            key, prefix = variable.decode("utf-8", "strict"), value.decode("utf-8", "strict")
            match = re.fullmatch(r"url\.(.+)\.(insteadof|pushinsteadof)", key, re.IGNORECASE)
            if not separator or match is None or not prefix or any(char in key + prefix for char in "\0\r\n"):
                return None
            rewrites.append(UrlRewrite(prefix, match.group(1), match.group(2).casefold() == "pushinsteadof"))
    except UnicodeError:
        return None
    return rewrites


def _aliases_from_config(config: str | None) -> dict[str, str] | None:
    if config is None:
        return None
    try:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.read_string(config)
    except configparser.Error:
        return None
    sections = [section for section in parser.sections() if section.casefold() == "alias"]
    aliases = {key.casefold(): value for section in sections for key, value in parser[section].items()}
    return aliases if all(key and "=" not in key and "\n" not in value and "\r" not in value for key, value in aliases.items()) else None


def read_text(cwd: str, target: str) -> str | None:
    path = Path(target)
    return None if target == "-" else _text(path if path.is_absolute() else Path(cwd) / path)


def _config_owned(
    config: str | None, inline_rewrites: list[UrlRewrite] | None = None, push: bool = False,
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
