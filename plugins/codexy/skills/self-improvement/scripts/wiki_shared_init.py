#!/usr/bin/env python3
"""Create one repository Wiki and separate archive in an empty authorized location."""

from __future__ import annotations

import argparse
import os
from pathlib import Path

from wiki_archive import unlinked
from wiki_init import git
from wiki_paths import git_scopes


def initialize(
    directory: Path, repository_home: str | None = None
) -> tuple[Path, Path]:
    scopes = git_scopes(directory, repository_home)
    base = scopes["repository"].parent
    unlinked(base.parent, directory=True)
    if not os.access(base.parent, os.W_OK):
        raise ValueError("repository Wiki parent is not writable")
    if base.exists() or base.is_symlink():
        raise ValueError(
            "repository Wiki destination already exists; use explicit migration or an alternate"
        )
    if repository_home is not None:
        # Shared memory cannot depend on another disposable Git worktree.
        probe = git(base.parent, "rev-parse", "--show-toplevel")
        if probe.returncode == 0:
            raise ValueError("alternate repository Wiki must be outside Git worktrees")
        elif b"not a git repository" not in probe.stderr.lower():
            raise ValueError(
                "cannot establish whether alternate destination is inside Git"
            )
    base.mkdir(mode=0o700)
    wiki, archive = scopes["repository"], scopes["archive"]
    wiki.mkdir(mode=0o700)
    archive.mkdir(mode=0o700)
    (wiki / "raw").mkdir()
    (wiki / "wiki").mkdir()
    for name, content in {
        "_index.md": "# Index\n",
        "config.md": "# Wiki configuration\n",
        "log.md": "# Activity log\n\n- Initialized repository Wiki.\n",
    }.items():
        with (wiki / name).open("x", encoding="utf-8") as output:
            output.write(content)
    return wiki, archive


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="existing Git worktree directory")
    args = parser.parse_args()
    try:
        wiki, archive = initialize(
            args.directory, os.environ.get("CODEXY_REPOSITORY_DATA_HOME")
        )
    except (OSError, ValueError) as error:
        parser.exit(
            1,
            f"repository Wiki initialization incomplete; preserve existing data: {error}\n",
        )
    print(f"Wiki: {wiki}\nArchive: {archive}")


if __name__ == "__main__":
    main()
