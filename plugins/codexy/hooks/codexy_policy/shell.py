"""Conservative structural checks for sensitive shell operations."""

from __future__ import annotations

import os
import re
import shlex
from pathlib import Path

from .repository import OWNED, identity, repository_owned
from .titles import issue_title, pr_title

OPS = {";", "&&", "||", "|", "&"}
WRAPPERS = {"env", "command", "sudo", "exec"}
OPAQUE = re.compile(r"\$\(|`|\$\{|<<<?|\b(?:eval|if|for|while|until|case)\b")
SUBCOMMAND = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")


def forbidden(command: str, cwd: str, depth: int = 0) -> bool:
    cwd_owned = repository_owned(cwd)
    if depth > 3:
        return True
    if OPAQUE.search(command):
        if cwd_owned is not False:
            return True
        for match in SUBCOMMAND.finditer(command):
            nested = match.group(1) if match.group(1) is not None else match.group(2)
            if forbidden(nested, cwd, depth + 1):
                return True
    try:
        lexer = shlex.shlex(command.replace("\n", ";"), posix=True, punctuation_chars=";&|(){}")
        lexer.whitespace_split, lexer.commenters = True, ""
        tokens = list(lexer)
    except ValueError:
        return cwd_owned is not False
    if any(token in {"(", ")", "{", "}"} for token in tokens):
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
        if _segment(segment, cwd, cwd_owned, depth):
            return True
        if following == "|" and index + 1 < len(segments) and _name(segments[index + 1][0][0]) in {"sh", "bash", "zsh", "dash", "pwsh", "powershell"}:
            return cwd_owned is not False
    return False


def _segment(tokens: list[str], cwd: str, cwd_owned: bool | None, depth: int) -> bool:
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
        elif name == "sudo":
            tokens = _sudo_command(tokens)
            if tokens is None:
                return cwd_owned is not False
    if not tokens:
        return False
    name, args = _name(tokens[0]), tokens[1:]
    if name in {"sh", "bash", "zsh", "dash", "pwsh", "powershell", "cmd"}:
        for index, arg in enumerate(args):
            if arg.lower() in {"-c", "-command", "/c"}:
                return index + 1 >= len(args) or forbidden(args[index + 1], cwd, depth + 1)
        return cwd_owned is not False
    if name == "xargs":
        return _xargs(args, cwd, cwd_owned, depth)
    if name == "git":
        return _git(args, cwd, cwd_owned)
    if name == "gh":
        return _gh(args, cwd_owned)
    if name == "rm":
        return cwd_owned is not False and _rm(args)
    return False


def _git(args: list[str], cwd: str, cwd_owned: bool | None) -> bool:
    while args and args[0].startswith("-"):
        option = args[0]
        if option in {"-c", "--config-env", "--git-dir", "--work-tree"} or option.startswith(("-c=", "--config-env=", "--git-dir=", "--work-tree=")):
            return cwd_owned is not False
        if option == "-C":
            if len(args) < 2:
                return True
            cwd = os.path.abspath(os.path.join(cwd, args[1]))
            cwd_owned = repository_owned(cwd)
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
        forced = any(arg in {"--force", "--force-with-lease", "--mirror"} or arg.startswith(("--force=", "--force-with-lease=", "--mirror=")) or (arg.startswith("-") and not arg.startswith("--") and "f" in arg[1:]) or arg.startswith("+") for arg in rest)
        return applies and forced
    return applies and ((operation == "reset" and "--hard" in rest) or (operation == "clean" and _flag(rest, "f", "--force")))


def _gh(args: list[str], cwd_owned: bool | None) -> bool:
    owned = cwd_owned
    filtered, index = [], 0
    while index < len(args):
        arg = args[index]
        if arg in {"-R", "--repo"}:
            if index + 1 >= len(args):
                return owned is not False
            owned = identity("https://github.com/" + args[index + 1]) == OWNED
            index += 2
        elif arg.startswith("--repo="):
            owned = identity("https://github.com/" + arg.split("=", 1)[1]) == OWNED
            index += 1
        else:
            filtered.append(arg)
            index += 1
    if owned is False:
        return False
    operation = filtered[:2]
    if operation == ["pr", "merge"]:
        return any(arg == "--admin" or arg.startswith("--admin=") for arg in filtered[2:])
    if operation in (["pr", "create"], ["pr", "edit"]):
        present, title = _option_value(filtered[2:], "--title")
        return (operation[1] == "create" and not present) or (present and not pr_title(title))
    if operation in (["issue", "create"], ["issue", "edit"]):
        present, title = _option_value(filtered[2:], "--title")
        return (operation[1] == "create" and not present) or (present and not issue_title(title))
    return False


def _sudo_command(args: list[str]) -> list[str] | None:
    value_options = {"-u", "--user", "-g", "--group", "-h", "--host", "-p", "--prompt", "-C", "--close-from", "-D", "--chdir", "-R", "--chroot", "-T", "--command-timeout"}
    flag_options = {"-A", "--askpass", "-b", "--background", "-E", "--preserve-env", "-H", "--set-home", "-K", "--remove-timestamp", "-k", "--reset-timestamp", "-n", "--non-interactive", "-S", "--stdin", "-V", "--version", "-v", "--validate"}
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "--":
            return args[1:]
        if option in value_options:
            if len(args) < 2:
                return None
            args = args[2:]
        elif option in flag_options or option.startswith(tuple(item + "=" for item in value_options if item.startswith("--"))):
            args = args[1:]
        elif len(option) > 2 and option[:2] in {"-u", "-g", "-h", "-p", "-C", "-D", "-R", "-T"}:
            args = args[1:]
        else:
            return None
    return args


def _xargs(args: list[str], cwd: str, cwd_owned: bool | None, depth: int) -> bool:
    value_options = {"-a", "--arg-file", "-d", "--delimiter", "-E", "--eof", "-I", "--replace", "-L", "--max-lines", "-n", "--max-args", "-P", "--max-procs", "-s", "--max-chars"}
    flag_options = {"-0", "--null", "-o", "--open-tty", "-p", "--interactive", "-r", "--no-run-if-empty", "-t", "--verbose", "-x", "--exit"}
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "--":
            args = args[1:]
            break
        if option in value_options:
            if len(args) < 2:
                return cwd_owned is not False
            args = args[2:]
        elif option in flag_options or option.startswith(tuple(item + "=" for item in value_options if item.startswith("--"))):
            args = args[1:]
        else:
            return cwd_owned is not False
    return bool(args) and _segment(args, cwd, cwd_owned, depth + 1)


def _option_value(args: list[str], option: str) -> tuple[bool, str | None]:
    for index, arg in enumerate(args):
        if arg == option:
            return True, args[index + 1] if index + 1 < len(args) else None
        if arg.startswith(option + "="):
            return True, arg.split("=", 1)[1]
    return False, None


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
