"""Effective Git alias collection."""

from __future__ import annotations

import configparser
import subprocess
from collections.abc import Callable


def collect(
    cwd: str, git_dir: str | None, load_config: Callable[[str, str | None], str | None]
) -> dict[str, str] | None:
    command = ["git", "-C", cwd]
    if git_dir is not None:
        command.append(f"--git-dir={git_dir}")
    command.extend(["config", "--includes", "--null", "--get-regexp", r"^alias\."])
    try:
        result = subprocess.run(command, capture_output=True, check=False, timeout=1)
    except (OSError, subprocess.SubprocessError):
        return None
    if result.returncode not in {0, 1} or len(result.stdout) > 65536:
        return None
    aliases: dict[str, str] = {}
    try:
        for record in (item for item in result.stdout.split(b"\0") if item):
            variable, separator, value = record.partition(b"\n")
            key, command_text = (
                variable.decode("utf-8", "strict").casefold(),
                value.decode("utf-8", "strict"),
            )
            alias = key.removeprefix("alias.")
            if (
                not separator
                or not key.startswith("alias.")
                or not alias
                or "=" in alias
                or any(char in command_text for char in "\0\r\n")
            ):
                return None
            aliases[alias] = command_text
    except UnicodeError:
        return None
    local = from_config(load_config(cwd, git_dir))
    if local is None:
        return None
    aliases.update(local)
    return aliases


def from_config(config: str | None) -> dict[str, str] | None:
    if config is None:
        return None
    try:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.read_string(config)
    except configparser.Error:
        return None
    aliases = {
        key.casefold(): value
        for section in parser.sections()
        if section.casefold() == "alias"
        for key, value in parser[section].items()
    }
    return (
        aliases
        if all(
            key and "=" not in key and "\n" not in value and "\r" not in value
            for key, value in aliases.items()
        )
        else None
    )
