from __future__ import annotations

import shlex
from pathlib import Path

from .repository_policy import repository_owned

OPERATORS = {";", "&&", "||", "|", "&"}
SHELLS = {"sh", "bash", "zsh", "dash", "pwsh", "powershell"}
ENV_SWITCHES = {"-i", "-0", "--ignore-environment", "--null"}
ENV_VALUE_SWITCHES = {"-u", "-C", "--unset", "--chdir"}
SUDO_SWITCHES = {"-b", "-E", "-H", "-K", "-k", "-n", "-S"}
SUDO_VALUE_SWITCHES = {
    "-C", "-D", "-g", "-h", "-r", "-R", "-t", "-T", "-u",
    "--chdir", "--chroot", "--closefrom", "--group", "--host", "--role",
    "--type", "--command-timeout", "--user",
}
GIT_SWITCHES = {
    "--bare", "--literal-pathspecs", "--glob-pathspecs", "--noglob-pathspecs",
    "--icase-pathspecs", "--no-lazy-fetch", "--no-optional-locks", "--no-pager",
    "--no-replace-objects", "--paginate", "--version",
}
GIT_VALUE_SWITCHES = {
    "-C", "-c", "--config-env", "--exec-path", "--git-dir", "--namespace",
    "--super-prefix", "--work-tree",
}
GH_SWITCHES = {"--help", "--no-pager", "--paginate", "--version"}
GH_VALUE_SWITCHES = {"-R", "--config", "--hostname", "--repo"}


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


def _required_value(tokens: list[str], index: int) -> tuple[str, int] | None:
    if index + 1 >= len(tokens) or tokens[index + 1] == "--":
        return None
    return tokens[index + 1], index + 2


def _option_value(token: str, options: set[str]) -> bool:
    return any(token.startswith(f"{option}=") and token != f"{option}=" for option in options)


def _env_command(tokens: list[str]) -> list[str] | None:
    index = 1
    while index < len(tokens):
        token = tokens[index]
        if token == "--":
            return tokens[index + 1 :]
        if not token.startswith("-"):
            if "=" in token and not token.startswith("="):
                index += 1
                continue
            return tokens[index:]
        if token in ENV_SWITCHES or _option_value(token, {"--unset", "--chdir"}):
            index += 1
        elif token in ENV_VALUE_SWITCHES:
            value = _required_value(tokens, index)
            if value is None:
                return None
            _, index = value
        else:
            return None
    return []


def _sudo_command(tokens: list[str]) -> list[str] | None:
    index = 1
    while index < len(tokens):
        token = tokens[index]
        if token == "--":
            return tokens[index + 1 :]
        if not token.startswith("-"):
            return tokens[index:]
        if token in SUDO_SWITCHES or _option_value(token, {"--preserve-env"}):
            index += 1
        elif token in SUDO_VALUE_SWITCHES or _option_value(token, SUDO_VALUE_SWITCHES):
            value = _required_value(tokens, index)
            if value is None:
                return None
            _, index = value
        else:
            return None
    return []


def _command_command(tokens: list[str]) -> list[str] | None:
    index = 1
    while index < len(tokens):
        token = tokens[index]
        if token == "--":
            return tokens[index + 1 :]
        if token in {"-p", "-v", "-V"}:
            index += 1
            continue
        return None if token.startswith("-") else tokens[index:]
    return []


def _unwrap(tokens: list[str]) -> list[str] | None:
    remaining = list(tokens)
    for _ in range(8):
        if not remaining:
            return remaining
        command = Path(remaining[0]).name.lower()
        if command == "env":
            remaining = _env_command(remaining)
        elif command == "command":
            remaining = _command_command(remaining)
        elif command == "sudo":
            remaining = _sudo_command(remaining)
        else:
            return remaining
        if remaining is None:
            return None
    return None


def _global_options(
    tokens: list[str], switches: set[str], value_switches: set[str]
) -> list[str] | None:
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token == "--":
            return tokens[index + 1 :]
        if not token.startswith("-"):
            return tokens[index:]
        if token in switches or _option_value(token, value_switches):
            index += 1
        elif token in value_switches:
            value = _required_value(tokens, index)
            if value is None:
                return None
            _, index = value
        else:
            return None
    return []


def _has_flag(tokens: list[str], long: str, short: str = "") -> bool:
    for token in tokens:
        if token == long or token.startswith(f"{long}="):
            return True
        if short and token.startswith("-") and not token.startswith("--"):
            if any(option == short for option in token[1:] if option.isalpha()):
                return True
    return False


def _segment_forbidden(tokens: list[str], depth: int) -> bool:
    tokens = _unwrap(tokens)
    if tokens is None:
        return True
    if not tokens:
        return False
    command = Path(tokens[0]).name.lower()
    if command in SHELLS and depth < 3:
        lowered = [token.lower() for token in tokens]
        for selector in ("-c", "-command"):
            if selector in lowered:
                index = lowered.index(selector)
                return index + 1 >= len(tokens) or shell_forbidden(tokens[index + 1], depth + 1)
    if command == "git":
        arguments = _global_options(tokens[1:], GIT_SWITCHES, GIT_VALUE_SWITCHES)
        if arguments is None:
            return True
        if not arguments:
            return False
        operation, arguments = arguments[0], arguments[1:]
        if operation == "push":
            return _has_flag(arguments, "--force", "f") or _has_flag(
                arguments, "--force-with-lease"
            ) or any(argument.startswith("+") and ":" in argument for argument in arguments)
        if operation == "reset":
            return "--hard" in arguments
        if operation == "clean":
            return _has_flag(arguments, "--force", "f")
    if command == "gh":
        arguments = _global_options(tokens[1:], GH_SWITCHES, GH_VALUE_SWITCHES)
        if arguments is None:
            return True
        return arguments[:2] == ["pr", "merge"] and any(
            argument == "--admin" for argument in arguments[2:]
        )
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
