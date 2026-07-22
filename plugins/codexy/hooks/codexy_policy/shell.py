"""Conservative structural checks for sensitive shell operations."""

from __future__ import annotations

import re
import shlex
from pathlib import Path

from .repository import OWNED, identity

OPS = {";", "&&", "||", "|", "&"}
WRAPPERS = {"env", "command", "sudo", "exec"}
OPAQUE = re.compile(r"\$\(|`|\$\{|<<<?|\b(?:eval|if|for|while|until|case)\b")


def forbidden(command: str, cwd_owned: bool | None, depth: int = 0) -> bool:
    if depth > 3:
        return True
    if cwd_owned is not False and (OPAQUE.search(command) or any(c in command for c in "(){}")):
        return True
    try:
        lexer = shlex.shlex(command.replace("\n", ";"), posix=True, punctuation_chars=";&|")
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return cwd_owned is not False
    segments, current = [], []
    for token in tokens:
        if token in OPS:
            if current:
                segments.append((current, token))
                current = []
        else:
            current.append(token)
    if current:
        segments.append((current, ""))
    for index, (segment, following) in enumerate(segments):
        if _segment(segment, cwd_owned, depth):
            return True
        if following == "|" and index + 1 < len(segments) and _name(segments[index + 1][0][0]) in {"sh", "bash", "zsh", "dash", "pwsh", "powershell"}:
            return cwd_owned is not False
    return False


def _segment(tokens: list[str], cwd_owned: bool | None, depth: int) -> bool:
    while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
        tokens = tokens[1:]
    for _ in range(8):
        if not tokens or _name(tokens[0]) not in WRAPPERS:
            break
        name, tokens = _name(tokens[0]), tokens[1:]
        if name == "env":
            while tokens and (tokens[0].startswith("-") or "=" in tokens[0]):
                if tokens[0] in {"-u", "--unset", "-C", "--chdir"}:
                    if len(tokens) < 2:
                        return True
                    tokens = tokens[2:]
                else:
                    tokens = tokens[1:]
    if not tokens:
        return False
    name, args = _name(tokens[0]), tokens[1:]
    if name in {"sh", "bash", "zsh", "dash", "pwsh", "powershell", "cmd"}:
        for index, arg in enumerate(args):
            if arg.lower() in {"-c", "-command", "/c"}:
                return index + 1 >= len(args) or forbidden(args[index + 1], cwd_owned, depth + 1)
        return cwd_owned is not False
    if name == "git":
        return _git(args, cwd_owned)
    if name == "gh":
        return _gh(args, cwd_owned)
    if name == "rm":
        return cwd_owned is not False and _rm(args)
    return False


def _git(args: list[str], cwd_owned: bool | None) -> bool:
    while args and args[0].startswith("-"):
        option = args[0]
        if option in {"-c", "--config-env", "--git-dir", "--work-tree"} or option.startswith(("-c=", "--config-env=", "--git-dir=", "--work-tree=")):
            return cwd_owned is not False
        if option == "-C":
            if len(args) < 2:
                return True
            args = args[2:]
        elif option in {"--no-pager", "--paginate", "--bare"}:
            args = args[1:]
        else:
            return cwd_owned is not False
    if not args:
        return False
    operation, rest = args[0], args[1:]
    target_owned = _explicit_owned(rest)
    applies = target_owned is True or (target_owned is None and cwd_owned is not False)
    if operation == "push":
        forced = any(arg == "--force" or arg.startswith("--force=") or arg == "--force-with-lease" or arg.startswith("--force-with-lease=") or (arg.startswith("-") and not arg.startswith("--") and "f" in arg[1:]) or arg.startswith("+") for arg in rest)
        return applies and forced
    return applies and ((operation == "reset" and "--hard" in rest) or (operation == "clean" and _flag(rest, "f", "--force")))


def _gh(args: list[str], cwd_owned: bool | None) -> bool:
    owned = cwd_owned
    for index, arg in enumerate(args):
        if arg in {"-R", "--repo"} and index + 1 < len(args):
            owned = identity("https://github.com/" + args[index + 1]) == OWNED
        elif arg.startswith("--repo="):
            owned = identity("https://github.com/" + arg.split("=", 1)[1]) == OWNED
    return owned is not False and args[:2] == ["pr", "merge"] and any(arg == "--admin" or arg.startswith("--admin=") for arg in args[2:])


def _rm(args: list[str]) -> bool:
    targets = [arg for arg in args if not arg.startswith("-")]
    broad = {"/", "/*", "~", "$HOME", "${HOME}"}
    return _flag(args, "r", "--recursive") and _flag(args, "f", "--force") and any(target in broad or target.rstrip("/").endswith("/..") for target in targets)


def _explicit_owned(args: list[str]) -> bool | None:
    identities = [identity(arg) for arg in args if identity(arg) is not None]
    return None if not identities else OWNED in identities


def _flag(args: list[str], short: str, long: str) -> bool:
    return any(arg == long or arg.startswith(long + "=") or (arg.startswith("-") and not arg.startswith("--") and short in arg[1:]) for arg in args)


def _name(value: str) -> str:
    name = Path(value).name.lower()
    return name[:-4] if name.endswith(".exe") else name
