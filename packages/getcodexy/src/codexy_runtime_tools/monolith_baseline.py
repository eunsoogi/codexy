"""Fail-closed fingerprints for legacy Codexy plugin trees."""

from __future__ import annotations

import hashlib
import os
import stat
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Baseline:
    version: str
    tree_sha256: str


BASELINES = {
    "1.3.0": Baseline(
        "1.3.0", "9e1f2c8a97fe24949ea3fc11762246602c8b81b66b1298b88d5114bf71dc0b3b"
    )
}


def classify_tree(root: Path, baseline: Baseline) -> str:
    try:
        digest = tree_digest(root)
    except (OSError, ValueError):
        return "ambiguous"
    return "supported-unmodified" if digest == baseline.tree_sha256 else "modified"


def tree_digest(root: Path) -> str:
    root = Path(root)
    metadata = root.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or _link(metadata):
        raise ValueError("legacy plugin root must be a real directory")
    if os.name == "nt":
        raise ValueError("automatic legacy traversal is unavailable on Windows")
    records = []
    for directory, children, files, descriptor in os.fwalk(root, follow_symlinks=False):
        relative = Path(directory).relative_to(root)
        opened_directory = os.fstat(descriptor)
        if not stat.S_ISDIR(opened_directory.st_mode) or _reparse(opened_directory):
            raise ValueError("legacy plugin tree has an unsafe directory")
        records.append(
            f"D\0{relative.as_posix()}\0{opened_directory.st_mode & 0o777:o}"
        )
        children.sort()
        for name in sorted(children + files):
            entry = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
            if _link(entry) or _reparse(entry):
                raise ValueError("legacy plugin tree has an unsafe link")
            if stat.S_ISDIR(entry.st_mode):
                continue
            if not stat.S_ISREG(entry.st_mode) or entry.st_nlink != 1:
                raise ValueError("legacy plugin tree has an unsafe file")
            opened = os.open(
                name,
                os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0),
                dir_fd=descriptor,
            )
            try:
                checked = os.fstat(opened)
                if not stat.S_ISREG(checked.st_mode) or (
                    checked.st_dev,
                    checked.st_ino,
                ) != (entry.st_dev, entry.st_ino):
                    raise ValueError("legacy plugin file changed while reading")
                with os.fdopen(opened, "rb", closefd=False) as source:
                    digest = hashlib.sha256(source.read()).hexdigest()
            finally:
                os.close(opened)
            records.append(
                f"F\0{(relative / name).as_posix()}\0{entry.st_mode & 0o777:o}\0{digest}"
            )
    return hashlib.sha256("\n".join(records).encode()).hexdigest()


def _link(metadata: os.stat_result) -> bool:
    return stat.S_ISLNK(metadata.st_mode)


def _reparse(metadata: os.stat_result) -> bool:
    return bool(
        getattr(metadata, "st_file_attributes", 0)
        & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    )
