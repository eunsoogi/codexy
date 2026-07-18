from __future__ import annotations

import configparser
import re
import shlex
from pathlib import Path


CODEXY_REMOTE = re.compile(r"(?:github\.com[/:])eunsoogi/codexy(?:\.git)?$")
OPERATORS = {";", "&&", "||", "|", "&"}
SHELLS = {"sh", "bash", "zsh", "dash", "pwsh", "powershell"}


def _repository_config(cwd: Path) -> Path | None:
    for root in (cwd, *cwd.parents):
        dot_git = root / ".git"
        if dot_git.is_dir():
            return dot_git / "config"
        if dot_git.is_file():
            text = dot_git.read_text(encoding="utf-8", errors="strict").strip()
            prefix = "gitdir: "
            if not text.startswith(prefix):
                return None
            git_dir = Path(text[len(prefix) :])
            if not git_dir.is_absolute():
                git_dir = dot_git.parent / git_dir
            common = git_dir / "commondir"
            if common.is_file():
                common_dir = Path(common.read_text(encoding="utf-8").strip())
                if not common_dir.is_absolute():
                    common_dir = git_dir / common_dir
                return common_dir.resolve() / "config"
            return git_dir.resolve() / "config"
    return None


def repository_owned(cwd: str) -> bool | None:
    path = Path(cwd)
    if not path.is_absolute():
        return None
    try:
        config_path = _repository_config(path)
        if config_path is None or not config_path.is_file():
            return None if path.name == "codexy" else False
        config = configparser.ConfigParser(interpolation=None)
        config.read(config_path, encoding="utf-8")
        urls = [
            section.get("url", "")
            for name in config.sections()
            if name.startswith('remote "')
            for section in [config[name]]
        ]
        return any(CODEXY_REMOTE.search(url) for url in urls)
    except (OSError, UnicodeError, configparser.Error, ValueError):
        return None


def _tokens(command: str) -> list[str]:
    lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|")
    lexer.whitespace_split = True
    lexer.commenters = ""
    return list(lexer)


def _segments(tokens: list[str]) -> list[list[str]]:
    result: list[list[str]] = []
    current: list[str] = []
    for token in tokens:
        if token in OPERATORS:
            if current:
                result.append(current)
                current = []
        else:
            current.append(token)
    if current:
        result.append(current)
    return result


def _unwrap(tokens: list[str]) -> list[str]:
    remaining = list(tokens)
    while remaining:
        command = Path(remaining[0]).name.lower()
        if command == "env":
            remaining.pop(0)
            while remaining and ("=" in remaining[0] or remaining[0].startswith("-")):
                remaining.pop(0)
        elif command == "command":
            remaining.pop(0)
        elif command == "sudo":
            remaining.pop(0)
            while remaining and remaining[0].startswith("-"):
                remaining.pop(0)
        else:
            break
    return remaining


def _has_flag(tokens: list[str], long: str, short: str = "") -> bool:
    return any(
        token == long
        or token.startswith(f"{long}=")
        or (short and token.startswith("-") and not token.startswith("--") and short in token[1:])
        for token in tokens
    )


def _segment_forbidden(tokens: list[str], depth: int) -> bool:
    tokens = _unwrap(tokens)
    if not tokens:
        return False
    command = Path(tokens[0]).name.lower()
    if command in SHELLS and depth < 3:
        lowered = [token.lower() for token in tokens]
        for selector in ("-c", "-command"):
            if selector in lowered:
                index = lowered.index(selector)
                return index + 1 >= len(tokens) or shell_forbidden(tokens[index + 1], depth + 1)
    if command == "git" and len(tokens) >= 2:
        operation, arguments = tokens[1], tokens[2:]
        if operation == "push":
            return _has_flag(arguments, "--force", "f") or _has_flag(
                arguments, "--force-with-lease"
            ) or any(argument.startswith("+") and ":" in argument for argument in arguments)
        if operation == "reset":
            return "--hard" in arguments
        if operation == "clean":
            return _has_flag(arguments, "--force", "f")
    if command == "gh" and tokens[1:3] == ["pr", "merge"]:
        return "--admin" in tokens[3:]
    if command == "rm":
        recursive = _has_flag(tokens[1:], "--recursive", "r")
        force = _has_flag(tokens[1:], "--force", "f")
        targets = [token for token in tokens[1:] if not token.startswith("-")]
        return recursive and force and any(target in {"/", "~", "$HOME"} for target in targets)
    return False


def shell_forbidden(command: str, depth: int = 0) -> bool:
    try:
        return any(_segment_forbidden(segment, depth) for segment in _segments(_tokens(command)))
    except ValueError:
        return True
