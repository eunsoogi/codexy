#!/usr/bin/env python3
"""Initialize a new global Wiki only at an explicitly authorized destination."""

from __future__ import annotations

import argparse
import os
import platform
from pathlib import Path
from typing import Mapping

from wiki_archive import unlinked
from wiki_init import git
from wiki_paths import global_wiki


def initialize(env: Mapping[str, str], system: str, home: Path) -> Path:
    root = global_wiki(env, system, home)
    if not root.is_absolute() or root.exists() or root.is_symlink():
        raise ValueError("global Wiki destination exists or is not absolute")
    missing = []
    parent = root.parent
    while not parent.exists() and not parent.is_symlink():
        missing.append(parent)
        parent = parent.parent
    unlinked(parent, directory=True)
    if not os.access(parent, os.W_OK):
        raise ValueError("global Wiki parent is not writable")
    probe = git(parent, "rev-parse", "--show-toplevel")
    if probe.returncode == 0:
        top = Path(os.fsdecode(probe.stdout).strip()).resolve(strict=True)
        relative = root.relative_to(top).as_posix()
        tracked = git(top, "ls-files", "-z", "--", relative)
        ignored = git(top, "check-ignore", "--no-index", "-q", relative + "/probe")
        if tracked.returncode or tracked.stdout or ignored.returncode:
            raise ValueError(
                "global Wiki inside Git must already be ignored and untracked"
            )
    elif b"not a git repository" not in probe.stderr.lower():
        raise ValueError("cannot establish whether global Wiki is inside Git")
    for directory in reversed(missing):
        directory.mkdir(mode=0o700)
    root.mkdir(mode=0o700)
    (root / "raw").mkdir()
    (root / "wiki").mkdir()
    for name, content in {
        "_index.md": "# Index\n",
        "config.md": "# Wiki configuration\n",
        "log.md": "# Activity log\n\n- Initialized global Wiki.\n",
    }.items():
        with (root / name).open("x", encoding="utf-8") as output:
            output.write(content)
    return root


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    try:
        print(initialize(os.environ, platform.system(), Path.home()))
    except (OSError, ValueError) as error:
        parser.exit(
            1,
            f"global Wiki initialization incomplete; preserve existing data: {error}\n",
        )


if __name__ == "__main__":
    main()
