"""Conservative structural checks for sensitive shell operations."""

from __future__ import annotations

import os
import re
import shlex
from pathlib import Path

from .github import forbidden as gh_forbidden
from .repository import OWNED, git_directory_owned, identity, repository_owned
from .titles import issue_title, pr_title
from .wrappers import command_command, sudo_command, time_command, timeout_command

OPS = {";", "&&", "||", "|", "&"}
WRAPPERS = {"env", "command", "sudo", "exec", "time", "timeout"}
OPAQUE = re.compile(r"\$\(|`|\$\{|<<<?|\b(?:eval|if|for|while|until|case)\b")
SUBCOMMAND = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")


def forbidden(command: str, cwd: str, depth: int = 0) -> bool:
    cwd_owned = repository_owned(cwd)
    if depth > 3:
        return True
    if OPAQUE.search(command):
        if cwd_owned is not False:
            return True
        try:
            opaque_tokens = shlex.split(command)
            if _explicit_owned(opaque_tokens) is True:
                return True
            if opaque_tokens and _name(opaque_tokens[0]) == "eval" and any(forbidden(arg, cwd, depth + 1) for arg in opaque_tokens[1:]):
                return True
        except ValueError:
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
    active_cwd = cwd
    for index, (segment, following) in enumerate(segments):
        active_owned = repository_owned(active_cwd)
        if _segment(segment, active_cwd, active_owned, depth):
            return True
        if following == "|" and index + 1 < len(segments) and _name(segments[index + 1][0][0]) in {"sh", "bash", "zsh", "dash", "pwsh", "powershell"}:
            return active_owned is not False
        changed_cwd = _changed_directory(segment, active_cwd)
        if following in {";", "&&"} or (following == "||" and (active_owned is False or repository_owned(changed_cwd) is not False)):
            active_cwd = changed_cwd
    return False


def _segment(tokens: list[str], cwd: str, cwd_owned: bool | None, depth: int) -> bool:
    git_dir_owned: bool | None = None
    gh_repo_owned: bool | None = None
    while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
        if tokens[0].startswith("GIT_DIR="):
            cwd_owned = git_dir_owned = git_directory_owned(cwd, tokens[0].split("=", 1)[1])
        elif tokens[0].startswith("GH_REPO="):
            gh_repo_owned = identity("https://github.com/" + tokens[0].split("=", 1)[1]) == OWNED
        tokens = tokens[1:]
    for _ in range(8):
        if not tokens or _name(tokens[0]) not in WRAPPERS:
            break
        name, tokens = _name(tokens[0]), tokens[1:]
        if name == "env":
            while tokens and (tokens[0].startswith("-") or "=" in tokens[0]):
                if tokens[0].startswith("GIT_DIR="):
                    cwd_owned = git_dir_owned = git_directory_owned(cwd, tokens[0].split("=", 1)[1])
                elif tokens[0].startswith("GH_REPO="):
                    gh_repo_owned = identity("https://github.com/" + tokens[0].split("=", 1)[1]) == OWNED
                if tokens[0] in {"-S", "--split-string"}:
                    return len(tokens) < 2 or forbidden(tokens[1], cwd, depth + 1)
                if tokens[0].startswith("--split-string="):
                    return forbidden(tokens[0].split("=", 1)[1], cwd, depth + 1)
                if tokens[0] in {"-u", "--unset", "-C", "--chdir"}:
                    if len(tokens) < 2:
                        return True
                    if tokens[0] in {"-C", "--chdir"}:
                        cwd = _resolve_cwd(cwd, tokens[1])
                        cwd_owned = git_dir_owned if git_dir_owned is not None else repository_owned(cwd)
                    tokens = tokens[2:]
                elif tokens[0].startswith("--chdir="):
                    cwd = _resolve_cwd(cwd, tokens[0].split("=", 1)[1])
                    cwd_owned = git_dir_owned if git_dir_owned is not None else repository_owned(cwd)
                    tokens = tokens[1:]
                else:
                    tokens = tokens[1:]
        elif name == "sudo":
            wrapped = tokens
            tokens = sudo_command(tokens)
            if tokens is None:
                return cwd_owned is not False or _explicit_owned(wrapped) is True
        elif name == "time":
            wrapped = tokens
            tokens = time_command(tokens)
            if tokens is None:
                return cwd_owned is not False or _explicit_owned(wrapped) is True
        elif name == "timeout":
            wrapped = tokens
            tokens = timeout_command(tokens)
            if tokens is None:
                return cwd_owned is not False or _explicit_owned(wrapped) is True
        elif name == "command":
            wrapped = tokens
            tokens = command_command(tokens)
            if tokens is None:
                return cwd_owned is not False or _explicit_owned(wrapped) is True
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
        return _git(args, cwd, cwd_owned, git_dir_owned)
    if name == "gh":
        return gh_forbidden(args, cwd_owned, gh_repo_owned)
    if name == "rm":
        return cwd_owned is not False and _rm(args)
    return False


def _git(args: list[str], cwd: str, cwd_owned: bool | None, git_dir_owned: bool | None) -> bool:
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "-c" or option.startswith("-c="):
            if option == "-c":
                if len(args) < 2:
                    return True
                config, args = args[1], args[2:]
            else:
                config, args = option[3:], args[1:]
            if cwd_owned is not False or _config_owned(config):
                return True
            continue
        if option in {"--config-env", "--git-dir", "--work-tree"} or option.startswith(("--config-env=", "--git-dir=", "--work-tree=")):
            return cwd_owned is not False or _explicit_owned(args) is True
        if option == "-C":
            if len(args) < 2:
                return True
            cwd = os.path.abspath(os.path.join(cwd, args[1]))
            cwd_owned = git_dir_owned if git_dir_owned is not None else repository_owned(cwd)
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
        elif option in flag_options or option.startswith(tuple(item + "=" for item in value_options if item.startswith("--"))) or any(option.startswith(short) and len(option) > len(short) for short in {"-a", "-d", "-E", "-I", "-L", "-n", "-P", "-s"}):
            args = args[1:]
        else:
            return cwd_owned is not False or _explicit_owned(args) is True
    return bool(args) and _segment(args, cwd, cwd_owned, depth + 1)


def _changed_directory(tokens: list[str], cwd: str) -> str:
    while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
        tokens = tokens[1:]
    if not tokens or _name(tokens[0]) != "cd":
        return cwd
    args = tokens[1:]
    if args[:1] == ["--"]:
        args = args[1:]
    return _resolve_cwd(cwd, args[0]) if len(args) == 1 else cwd


def _resolve_cwd(cwd: str, target: str) -> str:
    return os.path.abspath(os.path.join(cwd, target))


def _rm(args: list[str]) -> bool:
    targets = [arg for arg in args if not arg.startswith("-")]
    broad = {"/", "/*", "~", "$HOME", "${HOME}"}
    return _flag(args, "r", "--recursive") and _flag(args, "f", "--force") and any(target in broad or target.rstrip("/").endswith("/..") for target in targets)


def _explicit_owned(args: list[str]) -> bool | None:
    identities = [identity(arg) for arg in args if identity(arg) is not None]
    return None if not identities else OWNED in identities


def _config_owned(config: str) -> bool:
    return "=" in config and identity(config.split("=", 1)[1]) == OWNED


def _flag(args: list[str], short: str, long: str) -> bool:
    return any(arg == long or arg.startswith(long + "=") or (arg.startswith("-") and not arg.startswith("--") and short in arg[1:]) for arg in args)


def _name(value: str) -> str:
    name = Path(value).name.lower()
    return name[:-4] if name.endswith(".exe") else name
