"""Bounded reads of Git's effective include-aware remote configuration."""

from __future__ import annotations

import re
import subprocess

REMOTE_VALUE = re.compile(r"^remote\.(.+)\.(url|pushurl)$", re.IGNORECASE)


def remote_config(cwd: str, git_dir: str | None, push: bool) -> str | None:
    """Return a synthetic config containing every effective target URL."""
    command = ["git", "-C", cwd]
    if git_dir is not None:
        command.append(f"--git-dir={git_dir}")
    command.extend(
        ["config", "--includes", "--null", "--get-regexp", r"^remote\..*\.(url|pushurl)$"]
    )
    try:
        result = subprocess.run(command, capture_output=True, check=False, timeout=1)
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode not in {0, 1} or len(result.stdout) > 65536:
        return None
    remotes: dict[str, dict[str, list[str]]] = {}
    try:
        for record in (item for item in result.stdout.split(b"\0") if item):
            variable, separator, raw_value = record.partition(b"\n")
            key = variable.decode("utf-8", "strict")
            value = raw_value.decode("utf-8", "strict")
            match = REMOTE_VALUE.fullmatch(key)
            if not separator or match is None or not value or any(char in key + value for char in "\0\r\n"):
                return None
            remote = remotes.setdefault(match.group(1).casefold(), {"url": [], "pushurl": []})
            remote[match.group(2).casefold()].append(value)
    except UnicodeError:
        return None
    targets: list[str] = []
    for remote in remotes.values():
        targets.extend((remote["pushurl"] or remote["url"]) if push else remote["url"] + remote["pushurl"])
    return "".join(
        f'[remote "effective-{index}"]\n\turl = {value}\n'
        for index, value in enumerate(targets)
    )
