#!/usr/bin/env python3
"""Resolve Wiki scopes without creating or modifying user data."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
from pathlib import Path
from typing import Mapping

from wiki_archive import unlinked


def absolute(value: str, label: str) -> Path:
    path = Path(value).expanduser()
    if not path.is_absolute():
        raise ValueError(f"{label} must be an absolute path")
    return path


def global_wiki(env: Mapping[str, str], system: str, home: Path) -> Path:
    """Resolve the user data root; explicit configuration wins over defaults."""
    override = env.get("CODEXY_DATA_HOME")
    if override is not None:
        result = absolute(override, "CODEXY_DATA_HOME") / "wiki"
    elif system == "Darwin":
        result = home / "Library/Application Support/Codexy/wiki"
    elif system == "Linux":
        data = env.get("XDG_DATA_HOME")
        base = absolute(data, "XDG_DATA_HOME") if data else home / ".local/share"
        result = base / "codexy/wiki"
    elif system == "Windows":
        data = env.get("LOCALAPPDATA")
        if not data:
            raise ValueError("LOCALAPPDATA or CODEXY_DATA_HOME is required")
        result = absolute(data, "LOCALAPPDATA") / "Codexy/wiki"
    else:
        raise ValueError("unsupported platform: configure CODEXY_DATA_HOME")
    resolved = result.resolve()
    if resolved.is_relative_to(Path(__file__).resolve().parents[3]) or any(
        (parent / ".codex-plugin/plugin.json").is_file() for parent in resolved.parents
    ):
        raise ValueError("global Wiki must be outside the plugin distribution")
    return result


def git_value(cwd: Path, *args: str) -> Path:
    result = subprocess.run(
        ["git", "-C", str(cwd), "rev-parse", "--path-format=absolute", *args],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode or not result.stdout.strip():
        raise ValueError("cannot resolve Git identity in the requested directory")
    return absolute(result.stdout.strip(), "Git path")


def git_scopes(cwd: Path, repository_home: str | None = None) -> dict[str, Path]:
    """Use Git's common directory, not a remote URL or a worktree's .git file."""
    if not cwd.is_dir():
        raise ValueError("working directory must exist")
    root = git_value(cwd, "--show-toplevel").resolve(strict=True)
    common = git_value(cwd, "--git-common-dir").resolve(strict=True)
    if not root.is_dir() or not common.is_dir():
        raise ValueError("Git root and common directory must be directories")
    exclude = git_value(cwd, "--git-path", "info/exclude")
    if repository_home is not None:
        # A configured parent still partitions clones by their local Git identity.
        parent = absolute(repository_home, "CODEXY_REPOSITORY_DATA_HOME")
        storage = parent / hashlib.sha256(os.fsencode(str(common))).hexdigest()
        worktrees = subprocess.run(
            ["git", "-C", str(cwd), "worktree", "list", "--porcelain", "-z"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if worktrees.returncode:
            raise ValueError("cannot establish linked worktree boundaries")
        for entry in worktrees.stdout.split(b"\0"):
            if entry.startswith(b"worktree "):
                worktree = Path(os.fsdecode(entry[len(b"worktree ") :])).resolve()
                if storage.resolve().is_relative_to(worktree):
                    raise ValueError(
                        "repository Wiki must be outside every linked worktree"
                    )
    else:
        storage = common / "codexy"
    # The common Git directory survives linked-worktree retirement; no data is
    # created here and an existing destination is never assumed to be ours.
    return {
        "identity": common,
        "repository": storage / "wiki",
        "archive": storage / "task-archives",
        "worktree": root / ".wiki",
        "exclude": exclude,
    }


def standalone_scopes(directory: Path) -> dict[str, Path]:
    """For non-Git work, the explicitly selected directory is its identity."""
    if not directory.is_absolute() or not directory.is_dir():
        raise ValueError("standalone directory must be an existing absolute directory")
    unlinked(directory, directory=True)
    root = directory.resolve(strict=True)
    return {"identity": root, "worktree": root / ".wiki"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="existing task directory")
    parser.add_argument(
        "--standalone",
        action="store_true",
        help="use an explicit non-Git task directory; no repository scope",
    )
    args = parser.parse_args()
    try:
        scopes = (
            standalone_scopes(args.directory)
            if args.standalone
            else git_scopes(
                args.directory, os.environ.get("CODEXY_REPOSITORY_DATA_HOME")
            )
        )
        scopes["global"] = global_wiki(os.environ, platform.system(), Path.home())
    except (OSError, ValueError) as error:
        parser.exit(2, f"wiki path resolution failed: {error}\n")
    print(
        json.dumps({name: str(path) for name, path in scopes.items()}, sort_keys=True)
    )


if __name__ == "__main__":
    main()
