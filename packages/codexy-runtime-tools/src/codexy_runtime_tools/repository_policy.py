from __future__ import annotations

import configparser
import os
import re
import stat
from pathlib import Path
from urllib.parse import urlsplit


MAX_METADATA_DEPTH = 256
MAX_METADATA_BYTES = 64 * 1024
REPARSE_POINT = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
REMOTE_SECTION = re.compile(r'^remote "[^"\r\n]+"$')
SCP_REMOTE = re.compile(
    r"^(?:[A-Za-z0-9._-]+@)?(?P<host>[A-Za-z0-9.-]+):(?P<path>[^\s?#]+)$"
)


def _link_or_reparse(metadata: os.stat_result) -> bool:
    return stat.S_ISLNK(metadata.st_mode) or bool(
        getattr(metadata, "st_file_attributes", 0) & REPARSE_POINT
    )


def _metadata_chain(path: Path) -> os.stat_result | None:
    chain = [*reversed(path.parents), path]
    if not chain or len(chain) > MAX_METADATA_DEPTH:
        return None
    for index, component in enumerate(chain):
        try:
            metadata = os.lstat(component)
        except OSError:
            return None
        if _link_or_reparse(metadata):
            return None
        if index + 1 < len(chain) and not stat.S_ISDIR(metadata.st_mode):
            return None
    return metadata


def _regular_text(path: Path) -> str | None:
    metadata = _metadata_chain(path)
    if metadata is None or not stat.S_ISREG(metadata.st_mode):
        return None
    descriptor = None
    try:
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        opened = os.fstat(descriptor)
        if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
            return None
        with os.fdopen(descriptor, "rb", closefd=False) as stream:
            data = stream.read(MAX_METADATA_BYTES + 1)
        if len(data) > MAX_METADATA_BYTES:
            return None
        return data.decode("utf-8", errors="strict")
    except (OSError, UnicodeError):
        return None
    finally:
        if descriptor is not None:
            os.close(descriptor)


def _safe_directory(path: Path) -> bool:
    metadata = _metadata_chain(path)
    return metadata is not None and stat.S_ISDIR(metadata.st_mode)


def _metadata_directory(base: Path, raw: str) -> Path | None:
    if not raw or raw != raw.strip() or any(character in raw for character in "\0\r\n"):
        return None
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = base / candidate
    candidate = Path(os.path.abspath(candidate))
    return candidate if _safe_directory(candidate) else None


def _single_path(text: str, prefix: str = "") -> str | None:
    lines = text.splitlines()
    if len(lines) != 1 or not lines[0].startswith(prefix):
        return None
    value = lines[0][len(prefix) :]
    return value if value else None


def _git_directory(dot_git: Path) -> Path | None:
    text = _regular_text(dot_git)
    if text is None:
        return None
    raw = _single_path(text, "gitdir: ")
    return _metadata_directory(dot_git.parent, raw) if raw is not None else None


def _repository_config(cwd: Path) -> str | None:
    if not _safe_directory(cwd):
        return None
    for root in (cwd, *cwd.parents):
        dot_git = root / ".git"
        try:
            metadata = os.lstat(dot_git)
        except FileNotFoundError:
            continue
        except OSError:
            return None
        if _link_or_reparse(metadata):
            return None
        if stat.S_ISDIR(metadata.st_mode):
            return _regular_text(dot_git / "config")
        if not stat.S_ISREG(metadata.st_mode):
            return None
        git_dir = _git_directory(dot_git)
        if git_dir is None:
            return None
        common = git_dir / "commondir"
        try:
            os.lstat(common)
        except FileNotFoundError:
            return _regular_text(git_dir / "config")
        except OSError:
            return None
        common_text = _regular_text(common)
        raw = _single_path(common_text) if common_text is not None else None
        common_dir = _metadata_directory(git_dir, raw) if raw is not None else None
        return _regular_text(common_dir / "config") if common_dir is not None else None
    return None


def _remote_identity(url: str) -> tuple[str, str, str] | None:
    match = None if "://" in url else SCP_REMOTE.fullmatch(url)
    if match:
        host, path = match.group("host"), match.group("path")
    else:
        parsed = urlsplit(url)
        if (
            parsed.scheme not in {"http", "https", "ssh", "git"}
            or not parsed.hostname
            or parsed.password is not None
            or parsed.query
            or parsed.fragment
        ):
            return None
        host, path = parsed.hostname, parsed.path.lstrip("/")
    components = path.removesuffix(".git").split("/")
    if len(components) != 2 or not all(components):
        return None
    return host.lower(), components[0].lower(), components[1].lower()


def repository_owned(cwd: str) -> bool | None:
    path = Path(cwd)
    if not path.is_absolute():
        return None
    text = _repository_config(path)
    if text is None:
        return None
    try:
        config = configparser.ConfigParser(interpolation=None, strict=True)
        config.read_string(text)
        urls = [
            config[name].get("url", "")
            for name in config.sections()
            if REMOTE_SECTION.fullmatch(name)
        ]
    except configparser.Error:
        return None
    identities = [_remote_identity(url) for url in urls]
    if not identities or any(identity is None for identity in identities):
        return None
    consensus = set(identities)
    if len(consensus) != 1:
        return None
    return consensus == {("github.com", "eunsoogi", "codexy")}
