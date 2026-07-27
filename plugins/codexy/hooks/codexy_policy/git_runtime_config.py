"""Bounded reads of Git's effective include-aware remote configuration."""

from __future__ import annotations

import configparser
from io import StringIO
import re
import subprocess

REMOTE_VALUE = re.compile(r"^remote\.(.+)\.(url|pushurl)$", re.IGNORECASE)
REMOTE_SECTION = re.compile(r'^remote "([^"\r\n]+)"$', re.IGNORECASE)


def apply_remote_urls(config: str | None, remote_urls: tuple[tuple[str, str, str], ...]) -> str | None:
    """Apply tracked remote URL changes to a parsed local configuration."""
    if config is None:
        return None
    if not remote_urls:
        return config
    try:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.read_string(config)
        sections = {
            match.group(1).casefold(): section
            for section in parser.sections()
            if (match := REMOTE_SECTION.fullmatch(section)) is not None
        }
        for name, key, value in remote_urls:
            section = sections.get(name)
            if section is None:
                section = f'remote "{name}"'
                parser.add_section(section)
                sections[name] = section
            parser[section][key] = value
        output = StringIO()
        parser.write(output)
        return output.getvalue()
    except configparser.Error:
        return None


def remote_config(cwd: str, git_dir: str | None, push: bool, remote_urls: tuple[tuple[str, str, str], ...] = ()) -> str | None:
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
    for name, key, value in remote_urls:
        remote = remotes.get(name)
        if remote is None:
            return None
        remote[key] = [value]
    targets: list[str] = []
    for remote in remotes.values():
        targets.extend((remote["pushurl"] or remote["url"]) if push else remote["url"] + remote["pushurl"])
    return "".join(
        f'[remote "effective-{index}"]\n\turl = {value}\n'
        for index, value in enumerate(targets)
    )
