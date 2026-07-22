"""Conservative structural checks for sensitive shell operations."""

from __future__ import annotations

import re
import shlex

from .git_command import normalize as normalize_git
from .github import forbidden as gh_forbidden
from .repository import OWNED, git_directory_owned, github_identity, identity, repository_owned
from .shell_context import changed_directory as _changed_directory, command_option as _command_option, flag as _flag, name as _name, resolve_cwd as _resolve_cwd
from .titles import issue_title, pr_title
from .wrappers import command_command, exec_command, nohup_command, sudo_command, sudo_directory, time_command, timeout_command

OPS = {";", "&&", "||", "|", "&"}
WRAPPERS = {"env", "command", "sudo", "exec", "nohup", "time", "timeout"}
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
            if opaque_tokens and _name(opaque_tokens[0]) == "eval":
                evaluated = opaque_tokens[1:]
                if evaluated[:1] == ["--"]:
                    evaluated = evaluated[1:]
                if forbidden(" ".join(evaluated), cwd, depth + 1):
                    return True
            elif _explicit_owned(opaque_tokens) is True:
                return True
        except ValueError:
            return True
        for match in SUBCOMMAND.finditer(command):
            nested = match.group(1) if match.group(1) is not None else match.group(2)
            if forbidden(nested, cwd, depth + 1):
                return True
        if re.search(r"\$\{|<<<?|\b(?:if|for|while|until|case)\b", command):
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
    git_dir: str | None = None
    gh_repo: str | None = None
    while tokens and "=" in tokens[0] and not tokens[0].startswith("-"):
        if tokens[0].startswith("GIT_DIR="):
            git_dir = tokens[0].split("=", 1)[1]
        elif tokens[0].startswith("GH_REPO="):
            gh_repo = tokens[0].split("=", 1)[1]
        tokens = tokens[1:]
    for _ in range(8):
        if not tokens or _name(tokens[0]) not in WRAPPERS:
            break
        name, tokens = _name(tokens[0]), tokens[1:]
        if name == "env":
            while tokens and (tokens[0].startswith("-") or "=" in tokens[0]):
                option = tokens[0]
                if option.startswith("GIT_DIR="):
                    git_dir = option.split("=", 1)[1]
                elif option.startswith("GH_REPO="):
                    gh_repo = option.split("=", 1)[1]
                if option in {"-S", "--split-string"}:
                    return len(tokens) < 2 or forbidden(tokens[1], cwd, depth + 1)
                if option.startswith("--split-string="):
                    return forbidden(option.split("=", 1)[1], cwd, depth + 1)
                attached = option[:2] if option.startswith(("-u", "-C")) and len(option) > 2 else option
                value = option[2:] if attached != option else None
                if attached in {"-u", "--unset", "-C", "--chdir"}:
                    if value is not None:
                        tokens = [attached, value, *tokens[1:]]
                    if len(tokens) < 2:
                        return True
                    if attached == "-u" or attached == "--unset":
                        if tokens[1] == "GIT_DIR":
                            git_dir = None
                        elif tokens[1] == "GH_REPO":
                            gh_repo = None
                    else:
                        cwd = _resolve_cwd(cwd, tokens[1])
                        cwd_owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
                    tokens = tokens[2:]
                elif option.startswith("--chdir="):
                    cwd = _resolve_cwd(cwd, option.split("=", 1)[1])
                    cwd_owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
                    tokens = tokens[1:]
                else:
                    tokens = tokens[1:]
        elif name == "sudo":
            directory = sudo_directory(tokens)
            wrapped = tokens
            tokens = sudo_command(tokens)
            if tokens is None:
                return cwd_owned is not False or _explicit_owned(wrapped) is True
            if directory is not None:
                cwd = _resolve_cwd(cwd, directory)
                cwd_owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
        elif name == "exec":
            tokens = exec_command(tokens)
            if tokens is None:
                return cwd_owned is not False
        elif name == "nohup":
            tokens = nohup_command(tokens)
            if tokens is None:
                return cwd_owned is not False
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
    if git_dir is not None:
        cwd_owned = git_directory_owned(cwd, git_dir)
    gh_repo_owned = github_identity(gh_repo) == OWNED if gh_repo is not None else None
    name, args = _name(tokens[0]), tokens[1:]
    if name in {"sh", "bash", "zsh", "dash", "pwsh", "powershell", "cmd"}:
        for index, arg in enumerate(args):
            if _command_option(arg):
                return index + 1 >= len(args) or forbidden(args[index + 1], cwd, depth + 1)
        return cwd_owned is not False
    if name == "xargs":
        return _xargs(args, cwd, cwd_owned, depth)
    if name == "git":
        return _git(args, cwd, cwd_owned, git_dir)
    if name == "gh":
        return gh_forbidden(args, cwd_owned, gh_repo_owned)
    if name == "rm":
        return cwd_owned is not False and _rm(args)
    return False


def _git(args: list[str], cwd: str, cwd_owned: bool | None, git_dir: str | None) -> bool:
    invocation = normalize_git(args, cwd, cwd_owned, git_dir, _config_owned)
    if invocation is None:
        return True
    if invocation.alias_command is not None:
        return forbidden(invocation.alias_command, cwd, 1)
    if invocation.operation is None:
        return False
    operation, rest, cwd_owned = invocation.operation, invocation.arguments, invocation.cwd_owned
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


def _rm(args: list[str]) -> bool:
    targets = [arg for arg in args if not arg.startswith("-")]
    broad = {"/", "/*", "~", "$HOME", "${HOME}"}
    return _flag(args, "r", "--recursive") and _flag(args, "f", "--force") and any(target in broad or target.rstrip("/").endswith("/..") for target in targets)


def _explicit_owned(args: list[str]) -> bool | None:
    identities = [identity(arg) for arg in args if identity(arg) is not None]
    return None if not identities else OWNED in identities


def _config_owned(config: str) -> bool:
    return "=" in config and identity(config.split("=", 1)[1]) == OWNED
