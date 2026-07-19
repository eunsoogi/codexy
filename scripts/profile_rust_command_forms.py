"""Recognize command forms relevant to the exact Rust workload gate."""

from __future__ import annotations

CARGO_GLOBAL_VALUE_OPTIONS = frozenset({"-C", "--color", "--config", "-Z"})
CARGO_NON_EXECUTING_OPTIONS = frozenset({"-h", "--help", "-V", "--version"})
SUDO_VALUE_OPTIONS = frozenset(
    {
        "-C",
        "--chdir",
        "-D",
        "--chroot",
        "-R",
        "-g",
        "--group",
        "-p",
        "--prompt",
        "-r",
        "--role",
        "-t",
        "--type",
        "-T",
        "--command-timeout",
        "-u",
        "--user",
    }
)
SUDO_NON_EXECUTING_OPTIONS = frozenset(
    {
        "-h",
        "--help",
        "-K",
        "--remove-timestamp",
        "-l",
        "--list",
        "-V",
        "--version",
        "-v",
        "--validate",
    }
)


def executable_name(token: str) -> str:
    return token.rsplit("/", 1)[-1]


def cargo_test_workload(tokens: list[str], invocation: tuple[str, ...]) -> bool:
    if not tokens or executable_name(tokens[0]) != "cargo":
        return False
    execution_arguments = tokens[1 : tokens.index("--")] if "--" in tokens else tokens[1:]
    return (
        not CARGO_NON_EXECUTING_OPTIONS.intersection(execution_arguments)
        and cargo_subcommand(execution_arguments) == "test"
        and set(invocation[2:]).issubset(execution_arguments)
    )


def cargo_subcommand(arguments: list[str]) -> str | None:
    index = 0
    while index < len(arguments):
        token = arguments[index]
        if token == "--":
            return None
        if token in CARGO_GLOBAL_VALUE_OPTIONS:
            index += 2
        elif token.startswith(("--color=", "--config=")):
            index += 1
        elif token.startswith(("-C", "-Z")) and len(token) > 2:
            index += 1
        elif token.startswith("-"):
            index += 1
        else:
            return token
    return None


def sudo_index(tokens: list[str], index: int) -> int:
    while index < len(tokens):
        option = tokens[index]
        if option == "--":
            return index + 1
        if option in SUDO_NON_EXECUTING_OPTIONS:
            return len(tokens)
        if option in SUDO_VALUE_OPTIONS:
            index += 2
        elif option.startswith(
            (
                "--chdir=",
                "--chroot=",
                "--group=",
                "--prompt=",
                "--role=",
                "--type=",
                "--command-timeout=",
                "--user=",
            )
        ):
            index += 1
        elif option.startswith("-") and option != "-":
            index += 1
        else:
            return index
    return index
