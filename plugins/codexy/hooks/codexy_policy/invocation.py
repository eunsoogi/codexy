"""Typed, fail-closed effective command resolution for admission policy."""

from __future__ import annotations

import shlex
from dataclasses import dataclass

from .repository import git_directory_owned, repository_owned
from .shell_context import command_option, name, resolve_cwd

MAX_WRAPPER_DEPTH = 8
INTERPRETERS = {"sh", "bash", "zsh", "dash", "pwsh", "powershell", "cmd", "python", "python3", "node", "perl", "ruby"}


@dataclass(frozen=True)
class WrapperGrammar:
    values: frozenset[str]
    flags: frozenset[str]


WRAPPER_GRAMMAR = {
    "nice": WrapperGrammar(frozenset({"-n", "--adjustment"}), frozenset({"--help", "--version"})),
    "time": WrapperGrammar(frozenset({"-f", "--format", "-o", "--output"}), frozenset({"-a", "--append", "-p", "--portability", "-v", "--verbose"})),
    "timeout": WrapperGrammar(frozenset({"-k", "--kill-after", "-s", "--signal"}), frozenset({"--foreground", "--preserve-status", "-v", "--verbose"})),
    "sudo": WrapperGrammar(frozenset({"-u", "--user", "-g", "--group", "-h", "--host", "-p", "--prompt", "-C", "--close-from", "-D", "--chdir", "-R", "--chroot", "-T", "--command-timeout"}), frozenset({"-A", "--askpass", "-b", "--background", "-E", "--preserve-env", "-H", "--set-home", "-K", "--remove-timestamp", "-k", "--reset-timestamp", "-n", "--non-interactive", "-S", "--stdin", "-V", "--version", "-v", "--validate"})),
}


@dataclass(frozen=True)
class Invocation:
    executable: str | None
    arguments: list[str]
    cwd: str
    cwd_owned: bool | None
    git_dir: str | None
    gh_repo: str | None
    script: str | None = None
    opaque: bool = False


def resolve(tokens: list[str], cwd: str, depth: int = 0) -> Invocation | None:
    """Resolve wrapper launchers and their context before policy classification."""
    if depth > MAX_WRAPPER_DEPTH:
        return None
    git_dir, gh_repo = None, None
    while tokens and _assignment(tokens[0]):
        git_dir, gh_repo = _assign(tokens[0], git_dir, gh_repo)
        tokens = tokens[1:]
    owned = repository_owned(cwd)
    return _unwrap(tokens, cwd, owned, git_dir, gh_repo, depth)


def _unwrap(tokens: list[str], cwd: str, owned: bool | None, git_dir: str | None, gh_repo: str | None, depth: int) -> Invocation | None:
    for _ in range(MAX_WRAPPER_DEPTH):
        if not tokens:
            return Invocation(None, [], cwd, owned, git_dir, gh_repo)
        executable = name(tokens[0])
        args = tokens[1:]
        if executable == "env":
            result = _env(args, cwd, owned, git_dir, gh_repo)
            if result is None:
                return None
            tokens, cwd, owned, git_dir, gh_repo, script = result
            if script is not None:
                return Invocation(None, [], cwd, owned, git_dir, gh_repo, script=script)
            continue
        if executable in {"nice", "time", "sudo"}:
            result = _options(executable, args)
            if result is None:
                return None
            tokens, values = result
            if executable == "sudo" and (directory := values.get("-D") or values.get("--chdir")) is not None:
                cwd = resolve_cwd(cwd, directory)
                owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
            continue
        if executable == "timeout":
            result = _options(executable, args)
            if result is None or len(result[0]) < 2:
                return None
            tokens = result[0][1:]
            continue
        if executable == "command":
            result = _command(args)
            if result is None:
                return None
            tokens = result
            continue
        if executable == "exec":
            result = _exec(args)
            if result is None:
                return None
            tokens = result
            continue
        if executable == "nohup":
            tokens = args[1:] if args[:1] == ["--"] else args
            continue
        if executable == "xargs":
            result = _xargs(args)
            if result is None:
                return None
            return _unwrap(result, cwd, owned, git_dir, gh_repo, depth + 1)
        if executable in INTERPRETERS:
            for index, argument in enumerate(args):
                if command_option(argument):
                    return Invocation(executable, args, cwd, owned, git_dir, gh_repo, script=args[index + 1] if index + 1 < len(args) else "")
            return Invocation(executable, args, cwd, owned, git_dir, gh_repo, opaque=True)
        owned = git_directory_owned(cwd, git_dir) if git_dir is not None else owned
        return Invocation(executable, args, cwd, owned, git_dir, gh_repo)
    return None


def _assignment(value: str) -> bool:
    return "=" in value and not value.startswith("-")


def _assign(value: str, git_dir: str | None, gh_repo: str | None) -> tuple[str | None, str | None]:
    key, assigned = value.split("=", 1)
    return (assigned, gh_repo) if key == "GIT_DIR" else (git_dir, assigned) if key == "GH_REPO" else (git_dir, gh_repo)


def _env(args: list[str], cwd: str, owned: bool | None, git_dir: str | None, gh_repo: str | None) -> tuple[list[str], str, bool | None, str | None, str | None, str | None] | None:
    while args and (args[0].startswith("-") or _assignment(args[0])):
        option = args[0]
        if _assignment(option):
            git_dir, gh_repo = _assign(option, git_dir, gh_repo)
            args = args[1:]
        elif option == "--":
            args = args[1:]
            break
        elif option in {"-S", "--split-string"}:
            return ([], cwd, owned, git_dir, gh_repo, args[1] if len(args) > 1 else "")
        elif option.startswith("--split-string="):
            return ([], cwd, owned, git_dir, gh_repo, option.split("=", 1)[1])
        elif option in {"-u", "--unset", "-C", "--chdir"} or (option.startswith(("-u", "-C")) and len(option) > 2):
            attached = option[:2] if len(option) > 2 and option[:2] in {"-u", "-C"} else option
            value = option[2:] if attached != option else args[1] if len(args) > 1 else None
            if value is None:
                return None
            if attached in {"-u", "--unset"}:
                git_dir = None if value == "GIT_DIR" else git_dir
                gh_repo = None if value == "GH_REPO" else gh_repo
                if value == "GIT_DIR":
                    owned = repository_owned(cwd)
            else:
                cwd = resolve_cwd(cwd, value)
                owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
            args = args[1:] if attached != option else args[2:]
        elif option.startswith("--chdir="):
            cwd = resolve_cwd(cwd, option.split("=", 1)[1])
            owned = git_directory_owned(cwd, git_dir) if git_dir is not None else repository_owned(cwd)
            args = args[1:]
        elif option in {"-0", "--null", "-i", "--ignore-environment", "-v", "--debug"}:
            args = args[1:]
        else:
            return None
    return (args, cwd, owned, git_dir, gh_repo, None)


def _options(wrapper: str, args: list[str]) -> tuple[list[str], dict[str, str]] | None:
    grammar = WRAPPER_GRAMMAR[wrapper]
    values: dict[str, str] = {}
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "--":
            return args[1:], values
        matched = next((item for item in grammar.values if option == item or option.startswith(item + "=") or (len(item) == 2 and option.startswith(item) and len(option) > 2)), None)
        if matched is not None:
            value = option[len(matched):].removeprefix("=") or (args[1] if len(args) > 1 else None)
            if value is None:
                return None
            values[matched] = value
            args = args[1:] if option != matched else args[2:]
        elif option in grammar.flags or any(option.startswith(item + "=") for item in grammar.flags if item.startswith("--")):
            args = args[1:]
        else:
            return None
    return args, values


def _command(args: list[str]) -> list[str] | None:
    while args and args[0].startswith("-"):
        if args[0] == "--":
            return args[1:]
        if len(args[0]) < 2 or any(char not in "pVv" for char in args[0][1:]):
            return None
        if "V" in args[0] or "v" in args[0]:
            return []
        args = args[1:]
    return args


def _exec(args: list[str]) -> list[str] | None:
    while args and args[0].startswith("-"):
        if args[0] == "--":
            return args[1:]
        if args[0] in {"-c", "-l"}:
            args = args[1:]
        elif args[0] == "-a":
            args = args[2:] if len(args) > 1 else []
        elif args[0].startswith("-a") and len(args[0]) > 2:
            args = args[1:]
        else:
            return None
    return args


def _xargs(args: list[str]) -> list[str] | None:
    values = {"-a", "--arg-file", "-d", "--delimiter", "-E", "--eof", "-I", "--replace", "-L", "--max-lines", "-n", "--max-args", "-P", "--max-procs", "-s", "--max-chars"}
    flags = {"-0", "--null", "-o", "--open-tty", "-p", "--interactive", "-r", "--no-run-if-empty", "-t", "--verbose", "-x", "--exit"}
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "--":
            return args[1:]
        if option in values:
            args = args[2:] if len(args) > 1 else []
        elif option in flags or option.startswith(tuple(item + "=" for item in values if item.startswith("--"))) or any(option.startswith(item) and len(option) > len(item) for item in values if len(item) == 2):
            args = args[1:]
        else:
            return None
    return args
