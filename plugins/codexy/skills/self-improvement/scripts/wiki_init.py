#!/usr/bin/env python3
"""Initialize a new issue-worktree Wiki after checking Git exclusion."""

from __future__ import annotations

import argparse
import os
import stat
import subprocess
from pathlib import Path

from wiki_archive import read_file, unlinked
from wiki_paths import git_scopes, standalone_scopes


def git(root: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", "-C", str(root), *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        env={**os.environ, "LC_ALL": "C"},
    )


def init(directory: Path) -> Path:
    scopes = git_scopes(directory, os.environ.get("CODEXY_REPOSITORY_DATA_HOME"))
    local, exclude = scopes["worktree"], scopes["exclude"]
    root = local.parent
    unlinked(root, directory=True)
    unlinked(exclude.parent, directory=True)
    if local.exists() or local.is_symlink():
        raise ValueError(
            "local Wiki destination already exists; use explicit migration or an alternate"
        )
    tracked = git(root, "ls-files", "-z", "--", ".wiki")
    if tracked.returncode or tracked.stdout:
        raise ValueError("local Wiki path is tracked or cannot be checked")
    if not os.access(root, os.W_OK) or not os.access(exclude.parent, os.W_OK):
        raise ValueError("local Wiki or exclusion parent is not writable")
    if exclude.exists() or exclude.is_symlink():
        unlinked(exclude, directory=False)
        if not os.access(exclude, os.W_OK):
            raise ValueError("Git exclusion file is not writable")
    existing = (
        read_file(exclude.parent, exclude.name).splitlines() if exclude.exists() else []
    )
    if b"/.wiki/" not in existing:
        # Appending one rule never replaces an existing user's exclude file.
        flags = os.O_WRONLY | os.O_APPEND | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0)
        fd = os.open(exclude, flags, 0o600)
        try:
            opened, current = os.fstat(fd), exclude.lstat()
            if (
                not stat.S_ISREG(opened.st_mode)
                or stat.S_ISLNK(current.st_mode)
                or (opened.st_dev, opened.st_ino) != (current.st_dev, current.st_ino)
            ):
                raise ValueError("Git exclusion file changed during validation")
            rule = b"\n/.wiki/\n"
            if os.write(fd, rule) != len(rule):
                raise OSError("incomplete exclusion append")
        finally:
            os.close(fd)
    if git(root, "check-ignore", "--no-index", "-q", ".wiki/probe").returncode:
        raise ValueError("Git exclusion could not be verified")
    create_local(local)
    tracked = git(root, "ls-files", "-z", "--", ".wiki")
    if tracked.returncode or tracked.stdout:
        raise ValueError("local Wiki became tracked during initialization")
    return local


def create_local(local: Path) -> None:
    # Exclusive creation refuses a pre-existing or concurrently created Wiki.
    local.mkdir(mode=0o700)
    (local / "raw").mkdir()
    (local / "wiki").mkdir()
    for name, content in {
        "_index.md": "# Index\n",
        "config.md": "# Wiki configuration\n",
        "log.md": "# Activity log\n\n- Initialized local Wiki.\n",
    }.items():
        with (local / name).open("x", encoding="utf-8") as output:
            output.write(content)


def init_standalone(directory: Path) -> Path:
    unlinked(directory, directory=True)
    local = standalone_scopes(directory)["worktree"]
    if local.exists() or local.is_symlink() or not os.access(directory, os.W_OK):
        raise ValueError("standalone Wiki destination exists or is not writable")
    probe = git(directory, "rev-parse", "--show-toplevel")
    if probe.returncode == 0:
        raise ValueError("Git worktrees must use Git-aware initialization")
    if b"not a git repository" not in probe.stderr.lower():
        raise ValueError("cannot establish standalone Git boundary")
    create_local(local)
    return local


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="existing Git worktree directory")
    parser.add_argument(
        "--standalone", action="store_true", help="explicit non-Git task directory"
    )
    args = parser.parse_args()
    try:
        print(
            init_standalone(args.directory) if args.standalone else init(args.directory)
        )
    except (OSError, ValueError) as error:
        parser.exit(
            1, f"Wiki initialization incomplete; preserve existing content: {error}\n"
        )


if __name__ == "__main__":
    main()
