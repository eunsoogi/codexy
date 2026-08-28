"""Safe component reads and private freezing for public activation."""

from __future__ import annotations

import json
import os
import stat
import tempfile
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from .updater import _absolute, _validate_real_path

MAX_COMPONENT_BYTES = 512 * 1024 * 1024


def verify_component(
    root: Path, name: str, version: str | None = None
) -> dict[Path, bytes]:
    contents, _ = _verify_component(root, name, version)
    return contents


def _verify_component(
    root: Path, name: str, version: str | None = None
) -> tuple[dict[Path, bytes], dict[Path, int]]:
    if name not in ("codexy", "codexy-github"):
        raise ValueError(f"unknown component integrity identity: {name}")
    root = _absolute(root)
    _validate_real_path(root, require_exists=True)
    manifest = Path(".codex-plugin/plugin.json")
    modes: dict[Path, int] = {}
    manifest_contents = _read_regular(root, manifest, MAX_COMPONENT_BYTES)
    _verify_manifest(manifest_contents, name, version)
    verified = {manifest: manifest_contents}
    _read_component(root, verified, modes)
    return verified, modes


def _read_component(
    root: Path, verified: dict[Path, bytes], modes: dict[Path, int]
) -> None:
    size = sum(map(len, verified.values()))
    pending = [Path()]
    while pending:
        relative_directory = pending.pop()
        directory = root / relative_directory
        metadata = directory.lstat()
        if stat.S_ISLNK(metadata.st_mode) or _has_windows_reparse_point(metadata):
            raise ValueError(
                "component integrity path must not traverse link or reparse point: "
                f"{directory}"
            )
        if not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(f"component integrity requires directory: {directory}")
        with os.scandir(directory) as entries:
            for entry in sorted(entries, key=lambda item: item.name):
                relative = relative_directory / entry.name
                metadata = entry.stat(follow_symlinks=False)
                if stat.S_ISLNK(metadata.st_mode) or _has_windows_reparse_point(
                    metadata
                ):
                    raise ValueError(
                        "component integrity path must not traverse link or "
                        f"reparse point: {root / relative}"
                    )
                if stat.S_ISDIR(metadata.st_mode):
                    pending.append(relative)
                elif stat.S_ISREG(metadata.st_mode):
                    if relative not in verified:
                        verified[relative] = _read_regular(
                            root, relative, MAX_COMPONENT_BYTES - size, modes
                        )
                        size += len(verified[relative])
                else:
                    raise ValueError(
                        f"component integrity requires regular files: {root / relative}"
                    )


@contextmanager
def frozen_component(
    root: Path, name: str, version: str | None = None
) -> Iterator[Path]:
    contents, modes = _verify_component(root, name, version)
    with tempfile.TemporaryDirectory(prefix=f"{name}-verified-") as temporary:
        target = Path(temporary)
        for relative, data in contents.items():
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            descriptor = os.open(
                destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
            )
            with os.fdopen(descriptor, "wb") as output:
                output.write(data)
            os.chmod(destination, 0o600 | (modes.get(relative, 0) & 0o111))
        yield target


def _verify_manifest(contents: bytes, name: str, version: str | None) -> None:
    try:
        manifest = json.loads(contents, object_pairs_hook=_unique_object)
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"component manifest is invalid JSON: {name}") from error
    if not isinstance(manifest, dict) or (
        manifest.get("name"),
        manifest.get("repository"),
    ) != (name, "https://github.com/eunsoogi/codexy"):
        raise ValueError(f"component manifest identity mismatch: {name}")
    if version is not None and manifest.get("version") != version:
        raise ValueError(f"component manifest version mismatch: {name}")


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_regular(
    root: Path,
    relative: Path,
    limit: int | None = None,
    modes: dict[Path, int] | None = None,
) -> bytes:
    if _uses_windows_directory_fallback():
        return _read_regular_windows(root, relative, limit, modes)
    return _read_regular_posix(root, relative, limit, modes)


def _uses_windows_directory_fallback() -> bool:
    return os.name == "nt"


def _read_regular_posix(
    root: Path,
    relative: Path,
    limit: int | None,
    modes: dict[Path, int] | None,
) -> bytes:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    directory_flags = flags | getattr(os, "O_DIRECTORY", 0)
    descriptor = os.open(root, directory_flags)
    try:
        for part in relative.parts[:-1]:
            next_descriptor = os.open(part, directory_flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        with os.fdopen(
            os.open(relative.name, flags, dir_fd=descriptor), "rb"
        ) as source:
            metadata = os.fstat(source.fileno())
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError(
                    f"component integrity requires regular files: {root / relative}"
                )
            if modes is not None:
                modes[relative] = stat.S_IMODE(metadata.st_mode)
            return _read_limited(source, limit)
    finally:
        os.close(descriptor)


def _read_regular_windows(
    root: Path,
    relative: Path,
    limit: int | None,
    modes: dict[Path, int] | None,
) -> bytes:
    target, _ = _windows_safe_path(root, relative)
    descriptor = os.open(target, os.O_RDONLY | getattr(os, "O_BINARY", 0))
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode):
            raise ValueError(f"component integrity requires regular files: {target}")
        _, final = _windows_safe_path(root, relative)
        if (opened.st_dev, opened.st_ino) != (final.st_dev, final.st_ino):
            raise ValueError(
                f"component integrity path changed while reading: {target}"
            )
        if modes is not None:
            modes[relative] = stat.S_IMODE(opened.st_mode)
        with os.fdopen(descriptor, "rb", closefd=False) as source:
            return _read_limited(source, limit)
    finally:
        os.close(descriptor)


def _read_limited(source, limit: int | None) -> bytes:
    contents = source.read() if limit is None else source.read(limit + 1)
    if limit is not None and len(contents) > limit:
        raise ValueError("component integrity size limit exceeded")
    return contents


def _windows_safe_path(root: Path, relative: Path) -> tuple[Path, os.stat_result]:
    if relative.is_absolute() or any(
        part in {"", ".", ".."} for part in relative.parts
    ):
        raise ValueError(f"component integrity path is invalid: {relative}")
    current = root
    _windows_regular_path(current, directory=True)
    for part in relative.parts[:-1]:
        current /= part
        _windows_regular_path(current, directory=True)
    target = current / relative.name
    return target, _windows_regular_path(target, directory=False)


def _windows_regular_path(path: Path, directory: bool) -> os.stat_result:
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or _has_windows_reparse_point(metadata):
        raise ValueError(
            f"component integrity path must not traverse link or reparse point: {path}"
        )
    if stat.S_ISDIR(metadata.st_mode) != directory:
        kind = "directory" if directory else "regular file"
        raise ValueError(f"component integrity requires {kind}: {path}")
    return metadata


def _has_windows_reparse_point(metadata: os.stat_result) -> bool:
    attribute = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return bool(getattr(metadata, "st_file_attributes", 0) & attribute)
