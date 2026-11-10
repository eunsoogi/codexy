#!/usr/bin/env python3
"""Commit derived Wiki revisions using a serialized, atomic current pointer.

This storage primitive does not validate lesson quality, authorize promotion,
or adopt rules. Callers must establish those conditions before writing.
"""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path

from wiki_archive import read_file, sync_directory, unlinked, write_new
from wiki_exclusive_rename import rename_new
from wiki_store_candidate import inspect_candidate
from wiki_store_support import digest, encoded, lock, permitted
from validate_continuation_brief import DEFAULT_BUDGET, validate, validate_transition


def initial_files(root: Path) -> dict[str, bytes]:
    files = {}
    for name in ("_index.md", "log.md", "config.md"):
        files[name] = read_file(root, name)
    for name in ("wiki", "raw"):
        directory = root / name
        unlinked(directory, directory=True)
        for path in directory.rglob("*"):
            unlinked(path, directory=path.is_dir() and not path.is_symlink())
            if path.is_file():
                relative = path.relative_to(root).as_posix()
                if not permitted(relative, existing=True):
                    raise ValueError(f"unsupported Wiki file: {relative}")
                files[relative] = read_file(root, relative)
    return files


def write_revision(
    control: Path,
    files: dict[str, bytes | Path],
    processed: dict,
    prior_hashes: dict[str, str] | None = None,
) -> tuple[str, str]:
    prior_hashes = prior_hashes or {}
    hashes = {
        name: digest(data) if isinstance(data, bytes) else prior_hashes[name]
        for name, data in sorted(files.items())
    }
    metadata = {"files": hashes, "processed": processed}
    revision = digest(encoded(metadata))
    directory = Path(tempfile.mkdtemp(prefix="revision-", dir=control / "revisions"))
    for name, data in files.items():
        path = directory / name
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(data, bytes):
            write_new(path, data)
        else:
            # Unchanged immutable evidence is linked, not reread or copied into
            # the active context on every consolidation.
            unlinked(data, directory=False)
            os.link(data, path, follow_symlinks=False)
    write_new(directory / "state.json", encoded(metadata))
    for parent, _, _ in os.walk(directory, topdown=False):
        sync_directory(Path(parent))
    sync_directory(control / "revisions")
    return revision, directory.name


def publish(control: Path, revision: str, directory: str) -> None:
    # All content is immutable before the single visible pointer replacement.
    with tempfile.NamedTemporaryFile(
        dir=control, prefix="pointer-", delete=False
    ) as output:
        output.write(encoded({"revision": revision, "directory": directory}))
        output.flush()
        os.fsync(output.fileno())
        temporary = Path(output.name)
    try:
        os.replace(temporary, control / "CURRENT")
        sync_directory(control)
    finally:
        temporary.unlink(missing_ok=True)


def initialize(root: Path) -> str:
    """Import one explicitly selected existing Wiki without changing its files."""
    unlinked(root, directory=True)
    files = initial_files(root)
    control = root / ".codexy"
    if control.exists() or control.is_symlink():
        raise ValueError("Wiki revision destination already exists")
    # Stage outside the original topic root. The final exclusive rename makes a
    # pre-publication failure leave the existing Wiki tree untouched.
    with tempfile.TemporaryDirectory(
        prefix=".wiki-import-", dir=root.parent
    ) as temporary:
        staged = Path(temporary) / "control"
        staged.mkdir(mode=0o700)
        (staged / "revisions").mkdir()
        (staged / "lock").write_bytes(b"0")
        revision, directory = write_revision(staged, files, {})
        publish(staged, revision, directory)
        if initial_files(root) != files:
            raise ValueError("source Wiki changed during import")
        rename_new(staged, control)
        sync_directory(root)
    return revision


def revision_metadata(root: Path) -> tuple[str, Path, dict]:
    control = root / ".codexy"
    unlinked(control, directory=True)
    pointer = json.loads(read_file(control, "CURRENT"))
    if not isinstance(pointer, dict) or set(pointer) != {"revision", "directory"}:
        raise ValueError("invalid current Wiki pointer")
    name, revision = pointer["directory"], pointer["revision"]
    if (
        not isinstance(name, str)
        or not name.startswith("revision-")
        or Path(name).name != name
    ):
        raise ValueError("invalid Wiki revision directory")
    directory = control / "revisions" / name
    unlinked(directory, directory=True)
    metadata = json.loads(read_file(directory, "state.json"))
    if not isinstance(metadata, dict) or set(metadata) != {"files", "processed"}:
        raise ValueError("invalid Wiki revision state")
    if not isinstance(metadata["files"], dict) or not isinstance(
        metadata["processed"], dict
    ):
        raise ValueError("invalid Wiki revision contents")
    if not isinstance(revision, str) or digest(encoded(metadata)) != revision:
        raise ValueError("Wiki revision identity mismatch")
    for filename, expected in metadata["files"].items():
        if not isinstance(filename, str) or not permitted(filename, existing=True):
            raise ValueError("invalid Wiki revision file")
        if not isinstance(expected, str) or len(expected) != 64:
            raise ValueError("invalid Wiki file digest")
    return revision, directory, metadata


def read_selected(
    directory: Path, metadata: dict, name: str, limit: int | None = None
) -> bytes:
    if name not in metadata["files"] or not permitted(name, existing=True):
        raise ValueError("Wiki source is unavailable")
    path = directory / name
    unlinked(path, directory=False)
    if limit is not None and (limit <= 0 or path.stat().st_size > limit):
        raise ValueError("Wiki source exceeds its read budget")
    data = read_file(directory, name, limit)
    if digest(data) != metadata["files"][name]:
        raise ValueError("Wiki revision content mismatch")
    return data


def read_current(root: Path) -> tuple[str, dict[str, bytes]]:
    revision, directory, metadata = revision_metadata(root)
    return revision, {
        name: read_selected(directory, metadata, name) for name in metadata["files"]
    }


def read_one(root: Path, name: str, limit: int = 4000) -> tuple[str, bytes]:
    revision, directory, metadata = revision_metadata(root)
    return revision, read_selected(directory, metadata, name, limit)


def commit(root: Path, expected: str, candidate: dict) -> str:
    """Commit an authorized candidate or reject a stale/conflicting revision."""
    unlinked(root, directory=True)
    identifier, sources, changes, operation, checkpoint, candidate_hash = (
        inspect_candidate(candidate)
    )
    control = root / ".codexy"
    unlinked(control, directory=True)
    with lock(control):
        revision, previous, metadata = revision_metadata(root)
        processed = metadata["processed"]
        if identifier in processed:
            if (
                not isinstance(processed[identifier], dict)
                or processed[identifier].get("hash") != candidate_hash
            ):
                raise ValueError("candidate identity conflicts with prior input")
            return revision
        if revision != expected:
            raise ValueError(
                "stale Wiki revision; preserve and re-evaluate the candidate"
            )
        brief_name = "wiki/continuation.json"
        if brief_name in changes:
            budget = candidate.get("brief_budget_bytes", DEFAULT_BUDGET)
            if isinstance(budget, bool) or not isinstance(budget, int) or budget <= 0:
                raise ValueError("brief budget must be a positive integer")
            current_brief = validate(changes[brief_name].encode("utf-8"), budget)
            if brief_name in metadata["files"]:
                prior = validate(
                    read_selected(previous, metadata, brief_name, budget), budget
                )
                validate_transition(prior, current_brief)
        files: dict[str, bytes | Path] = {
            name: previous / name for name in metadata["files"]
        }
        for name, content in changes.items():
            if name.startswith("raw/") and name in files:
                raise ValueError("raw evidence revisions are immutable")
            files[name] = content.encode("utf-8")
        previous_log = read_selected(previous, metadata, "log.md")
        files["log.md"] = (
            previous_log
            + (f"\n- {operation} (sources: {', '.join(sources)})\n").encode()
        )
        # The evidence checkpoint and the candidate advance in the same snapshot.
        processed[identifier] = {
            "hash": candidate_hash,
            "checkpoint": checkpoint,
            "sources": sources,
        }
        revision, directory = write_revision(
            control, files, processed, metadata["files"]
        )
        publish(control, revision, directory)
        return revision
