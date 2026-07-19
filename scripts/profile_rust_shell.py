"""Normalize shell command forms used by the Rust workflow contract."""

from __future__ import annotations

import re
import shlex

from profile_rust_command_forms import cargo_test_workload, executable_name, sudo_index
from profile_rust_shell_source import shell_commands, strip_heredoc_data
from profile_rust_substitutions import command_substitutions

ASSIGNMENT_WORD_PATTERN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=.*$")
VALUE_OPTIONS = frozenset({"-a", "--argv0", "-C", "--chdir", "-u", "--unset"})
TIMEOUT_VALUE_OPTIONS = frozenset({"-k", "--kill-after", "-s", "--signal"})
TIME_VALUE_OPTIONS = frozenset({"-f", "--format", "-o", "--output"})
SHELL_PREFIXES = frozenset({"!", "do", "elif", "else", "if", "then", "until", "while"})
SHELL_EXECUTABLES = frozenset({"bash", "dash", "ksh", "sh", "zsh"})


def invocation_count(command: str, invocation: tuple[str, ...]) -> int:
    direct = sum(
        matches_invocation(candidate, invocation)
        for tokens in shell_commands(command)
        for candidate in executable_token_sets(tokens)
    )
    return direct + sum(
        invocation_count(nested, invocation)
        for nested in command_substitutions(strip_heredoc_data(command))
    )


def executable_token_sets(tokens: list[str]) -> list[list[str]]:
    normalized = executable_tokens(tokens)
    candidates = [normalized]
    child = shell_child_command(normalized)
    if child is not None:
        candidates.extend(
            candidate
            for nested in shell_commands(child)
            for candidate in executable_token_sets(nested)
        )
    return candidates


def matches_invocation(tokens: list[str], invocation: tuple[str, ...]) -> bool:
    if invocation[:2] == ("cargo", "test"):
        return cargo_test_workload(tokens, invocation)
    return tokens[: len(invocation)] == list(invocation)


def executable_tokens(tokens: list[str]) -> list[str]:
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if ASSIGNMENT_WORD_PATTERN.fullmatch(token):
            index += 1
        elif token in SHELL_PREFIXES:
            index += 1
        elif executable_name(token) == "env":
            return env_tokens(tokens, index + 1)
        elif executable_name(token) == "command":
            index = command_index(tokens, index + 1)
        elif executable_name(token) == "exec":
            index = exec_index(tokens, index + 1)
        elif executable_name(token) == "timeout":
            index = timeout_index(tokens, index + 1)
        elif executable_name(token) == "time":
            index = option_index(tokens, index + 1, TIME_VALUE_OPTIONS)
        elif executable_name(token) == "nice":
            index = option_index(tokens, index + 1, frozenset({"-n", "--adjustment"}))
        elif executable_name(token) == "sudo":
            index = sudo_index(tokens, index + 1)
        else:
            return tokens[index:]
    return []


def shell_child_command(tokens: list[str]) -> str | None:
    if not tokens or executable_name(tokens[0]) not in SHELL_EXECUTABLES:
        return None
    for index, option in enumerate(tokens[1:], start=1):
        if option == "-c" or option.startswith("-") and "c" in option[1:]:
            return tokens[index + 1] if index + 1 < len(tokens) else None
        if not option.startswith("-") or option == "-":
            return None
    return None


def env_tokens(tokens: list[str], index: int) -> list[str]:
    while index < len(tokens):
        option = tokens[index]
        if option == "--":
            return executable_tokens(tokens[index + 1 :])
        if ASSIGNMENT_WORD_PATTERN.fullmatch(option):
            index += 1
        elif option in VALUE_OPTIONS:
            index += 2
        elif option in {"-S", "--split-string"}:
            return executable_tokens(["env"] + shlex.split(tokens[index + 1]) + tokens[index + 2 :])
        elif option.startswith("--split-string="):
            return executable_tokens(
                ["env"] + shlex.split(option.partition("=")[2]) + tokens[index + 1 :]
            )
        elif option.startswith("-S") and len(option) > 2:
            return executable_tokens(["env"] + shlex.split(option[2:]) + tokens[index + 1 :])
        elif option.startswith(("--chdir=", "--unset=", "--argv0=")):
            index += 1
        elif option.startswith("-") and option != "-":
            index += 1
        else:
            return executable_tokens(tokens[index:])
    return []


def command_index(tokens: list[str], index: int) -> int:
    while index < len(tokens):
        option = tokens[index]
        if option == "--":
            return index + 1
        if not option.startswith("-") or option == "-":
            return index
        flags = option[1:]
        if not flags or set(flags) - {"p", "v", "V"} or set(flags) & {"v", "V"}:
            return len(tokens)
        index += 1
    return index


def exec_index(tokens: list[str], index: int) -> int:
    while index < len(tokens):
        option = tokens[index]
        if option == "--":
            return index + 1
        if option == "-a":
            index += 2
        elif option.startswith("-a") and len(option) > 2:
            index += 1
        elif option in {"-c", "-l"}:
            index += 1
        else:
            return index
    return index


def timeout_index(tokens: list[str], index: int) -> int:
    while index < len(tokens):
        option = tokens[index]
        if option == "--":
            return index + 1
        if option in TIMEOUT_VALUE_OPTIONS:
            index += 2
        elif option.startswith(("--kill-after=", "--signal=")) or option.startswith("-"):
            index += 1
        else:
            return index + 1
    return index


def option_index(tokens: list[str], index: int, value_options: frozenset[str]) -> int:
    while index < len(tokens):
        option = tokens[index]
        if option == "--":
            return index + 1
        if option in value_options:
            index += 2
        elif option.startswith(("--format=", "--output=")) or option.startswith("-"):
            index += 1
        else:
            return index
    return index
