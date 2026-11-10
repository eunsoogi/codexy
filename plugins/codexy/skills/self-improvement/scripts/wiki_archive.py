#!/usr/bin/env python3
"""Preserve explicitly selected local Wiki files in a durable task archive."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import stat
import tempfile
from pathlib import Path


def write_new(path: Path, data: bytes) -> None:
    with path.open("xb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())
    # Revisions are append-only through this API; later commits link unchanged
    # files instead of modifying an earlier revision in place.
    if os.name != "nt":
        path.chmod(0o400)


def sync_directory(path: Path) -> None:
    # Windows does not expose portable directory fsync through Python.
    if os.name != "nt":
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            os.fsync(fd)
        finally:
            os.close(fd)


def unlinked(path: Path, *, directory: bool) -> None:
    """Refuse links and special files rather than follow an unexpected destination."""
    if not path.is_absolute():
        raise ValueError("paths must be absolute")
    for part in reversed((path, *path.parents)):
        info = part.lstat()
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"linked path is unsafe: {part}")
    mode = path.stat().st_mode
    if not (stat.S_ISDIR(mode) if directory else stat.S_ISREG(mode)):
        raise ValueError(f"unexpected file type: {path}")


def read_file(root: Path, relative: str, limit: int | None = None) -> bytes:
    name = Path(relative)
    if (
        name.is_absolute()
        or not name.parts
        or any(part in (".", "..") for part in name.parts)
    ):
        raise ValueError("archive selection must contain bounded relative file paths")
    source = root / name
    unlinked(source, directory=False)
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    fd = os.open(source, flags)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode):
            raise ValueError("archive selection must be a regular file")
        if limit is not None and limit <= 0:
            raise ValueError("read budget must be positive")
        with os.fdopen(fd, "rb", closefd=False) as handle:
            data = handle.read() if limit is None else handle.read(limit + 1)
        if limit is not None and len(data) > limit:
            raise ValueError("source exceeds its read budget")
        after = os.fstat(fd)
        if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) != (
            after.st_dev,
            after.st_ino,
            after.st_size,
            after.st_mtime_ns,
        ):
            raise ValueError("source changed while being read")
        return data
    finally:
        os.close(fd)


def archive(source: Path, destination: Path, files: list[str], origin: str) -> Path:
    """Publish an immutable archive through an exclusive atomic manifest link."""
    unlinked(source, directory=True)
    unlinked(destination, directory=True)
    if not files or len(files) != len(set(files)) or not origin.strip():
        raise ValueError("select distinct files and provide their task/revision origin")
    contents = {name: read_file(source, name) for name in sorted(files)}
    manifest = {
        "origin": origin,
        "files": {
            name: hashlib.sha256(data).hexdigest() for name, data in contents.items()
        },
    }
    identity = json.dumps(manifest, sort_keys=True, ensure_ascii=False).encode("utf-8")
    target = destination / (hashlib.sha256(identity).hexdigest() + ".json")
    stage = Path(tempfile.mkdtemp(prefix=".archive-data-", dir=destination))
    published = False
    try:
        for name, data in contents.items():
            output = stage / "files" / name
            output.parent.mkdir(parents=True, exist_ok=True)
            write_new(output, data)
        # A changed source invalidates the candidate before it becomes visible.
        if any(read_file(source, name) != data for name, data in contents.items()):
            raise ValueError("source changed before archive publication")
        manifest["data_dir"] = stage.name
        encoded = (
            json.dumps(manifest, sort_keys=True, ensure_ascii=False).encode("utf-8")
            + b"\n"
        )
        write_new(stage / "manifest.json", encoded)
        for parent, _, _ in os.walk(stage, topdown=False):
            sync_directory(Path(parent))
        sync_directory(destination)
        try:
            # Linking fails if any file already occupies the public name; no
            # unrelated destination can be overwritten by a racing writer.
            os.link(stage / "manifest.json", target)
            published = True
            sync_directory(destination)
        except FileExistsError:
            existing = json.loads(read_file(destination, target.name))
            if not isinstance(existing, dict):
                raise ValueError("existing archive manifest is invalid")
            folder = existing.pop("data_dir", "")
            if (
                not isinstance(folder, str)
                or not folder.startswith(".archive-data-")
                or Path(folder).name != folder
                or existing != manifest_without_dir(manifest)
            ):
                raise ValueError("archive destination conflicts with existing content")
            if any(
                read_file(destination / folder / "files", name) != data
                for name, data in contents.items()
            ):
                raise ValueError("existing archive failed content verification")
    finally:
        if not published:
            # Only the fresh directory created by this call is eligible for cleanup.
            shutil.rmtree(stage)
    verify_archive(target)
    return target


def manifest_without_dir(manifest: dict) -> dict:
    return {key: value for key, value in manifest.items() if key != "data_dir"}


def verify_archive(pointer: Path) -> None:
    """Verify the committed pointer and every selected file without source access."""
    unlinked(pointer.parent, directory=True)
    manifest = json.loads(read_file(pointer.parent, pointer.name))
    if not isinstance(manifest, dict) or set(manifest) != {
        "origin",
        "files",
        "data_dir",
    }:
        raise ValueError("archive manifest has an invalid shape")
    folder, files = manifest["data_dir"], manifest["files"]
    if (
        not isinstance(folder, str)
        or not folder.startswith(".archive-data-")
        or Path(folder).name != folder
        or not isinstance(files, dict)
        or not files
        or not isinstance(manifest["origin"], str)
        or not manifest["origin"].strip()
    ):
        raise ValueError("archive manifest contains an invalid origin or path")
    identity = json.dumps(
        manifest_without_dir(manifest), sort_keys=True, ensure_ascii=False
    ).encode("utf-8")
    if pointer.name != hashlib.sha256(identity).hexdigest() + ".json":
        raise ValueError("archive manifest identity mismatch")
    for name, digest in files.items():
        if not isinstance(name, str) or not isinstance(digest, str):
            raise ValueError("archive file entry is invalid")
        actual = hashlib.sha256(
            read_file(pointer.parent / folder / "files", name)
        ).hexdigest()
        if actual != digest:
            raise ValueError(f"archive content mismatch: {name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "source",
        type=Path,
        help="absolute local Wiki directory, or archive manifest with --verify",
    )
    parser.add_argument(
        "destination",
        nargs="?",
        type=Path,
        help="existing authorized archive directory",
    )
    parser.add_argument("--file", action="append", dest="files")
    parser.add_argument("--origin", help="task, branch and revision identity")
    parser.add_argument(
        "--verify",
        action="store_true",
        help="verify an existing archive without writing",
    )
    args = parser.parse_args()
    try:
        if args.verify:
            if args.destination is not None or args.files or args.origin:
                raise ValueError("--verify accepts only the archive manifest")
            verify_archive(args.source)
            print("archive verified")
        else:
            if args.destination is None or not args.files or not args.origin:
                raise ValueError("archive requires destination, --file and --origin")
            print(archive(args.source, args.destination, args.files, args.origin))
    except (OSError, ValueError) as error:
        parser.exit(1, f"archive failed; retain the worktree: {error}\n")


if __name__ == "__main__":
    main()
