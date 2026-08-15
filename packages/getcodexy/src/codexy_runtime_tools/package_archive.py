"""Safe extraction primitives for downloaded runtime archives."""

from __future__ import annotations

import stat
import tarfile
import zipfile
import zlib
from pathlib import Path

MAX_ARCHIVE_FILES = 2_048
MAX_UNPACKED_BYTES = 512 * 1024 * 1024


def _safe_extract_tar(archive: Path, destination: Path) -> None:
    try:
        _extract_tar(archive, destination)
    except (tarfile.TarError, EOFError) as error:
        raise ValueError(f"invalid runtime package archive: {error}") from error


def _extract_tar(archive: Path, destination: Path) -> None:
    destination_resolved = destination.resolve()
    with tarfile.open(archive, "r:gz") as package:
        members = package.getmembers()
        if len(members) > MAX_ARCHIVE_FILES:
            raise ValueError("runtime package contains too many members")
        if sum(member.size for member in members) > MAX_UNPACKED_BYTES:
            raise ValueError("runtime package exceeds the unpacked size limit")
        destinations: set[str] = set()
        for member in members:
            if not (member.isdir() or member.isfile()):
                raise ValueError(
                    f"runtime package contains unsafe link or device: {member.name}"
                )
            member_path = (destination / member.name).resolve()
            if (
                destination_resolved not in member_path.parents
                and member_path != destination_resolved
            ):
                raise ValueError(f"runtime package contains unsafe path: {member.name}")
            identity = str(member_path).casefold()
            if identity in destinations:
                raise ValueError(
                    f"runtime package contains duplicate path: {member.name}"
                )
            destinations.add(identity)
        package.extractall(destination)


def _safe_extract_zip(archive: Path, destination: Path) -> None:
    try:
        _extract_zip(archive, destination)
    except (zipfile.BadZipFile, zlib.error) as error:
        raise ValueError(f"invalid artifact archive: {error}") from error


def _extract_zip(archive: Path, destination: Path) -> None:
    destination_resolved = destination.resolve()
    with zipfile.ZipFile(archive) as zipped:
        members = zipped.infolist()
        if len(members) > MAX_ARCHIVE_FILES:
            raise ValueError("artifact archive contains too many members")
        if sum(member.file_size for member in members) > MAX_UNPACKED_BYTES:
            raise ValueError("artifact archive exceeds the unpacked size limit")
        destinations: set[str] = set()
        for member in members:
            member_path = (destination / member.filename).resolve()
            if stat.S_ISLNK(member.external_attr >> 16):
                raise ValueError(
                    f"artifact archive contains unsafe link: {member.filename}"
                )
            if (
                destination_resolved not in member_path.parents
                and member_path != destination_resolved
            ):
                raise ValueError(
                    f"artifact archive contains unsafe path: {member.filename}"
                )
            identity = str(member_path).casefold()
            if identity in destinations:
                raise ValueError(
                    f"artifact archive contains duplicate path: {member.filename}"
                )
            destinations.add(identity)
        zipped.extractall(destination)
