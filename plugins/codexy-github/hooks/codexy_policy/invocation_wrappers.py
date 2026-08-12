"""Parse bounded environment and launcher-wrapper grammar."""

from __future__ import annotations

import shlex
from dataclasses import dataclass

from .execution_context import ExecutionContext, assign, assignment, at, clear, unset
from .shell_context import resolve_cwd


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


def environment(args: list[str], context: ExecutionContext) -> tuple[list[str], ExecutionContext] | None:
    while args and (args[0].startswith("-") or assignment(args[0])):
        option = args[0]
        if assignment(option):
            context = assign(option, context)
            args = args[1:]
        elif option == "--":
            args = args[1:]
            break
        elif option in {"-S", "--split-string"}:
            if len(args) < 2:
                return None
            try:
                return shlex.split(args[1]) + args[2:], context
            except ValueError:
                return None
        elif option.startswith("--split-string="):
            try:
                return shlex.split(option.split("=", 1)[1]) + args[1:], context
            except ValueError:
                return None
        elif option in {"-u", "--unset", "-C", "--chdir"} or (option.startswith(("-u", "-C")) and len(option) > 2):
            attached = option[:2] if len(option) > 2 and option[:2] in {"-u", "-C"} else option
            value = option[2:] if attached != option else args[1] if len(args) > 1 else None
            if value is None:
                return None
            context = unset(context, value) if attached in {"-u", "--unset"} else at(context, resolve_cwd(context.cwd, value))
            args = args[1:] if attached != option else args[2:]
        elif option.startswith("--chdir="):
            context = at(context, resolve_cwd(context.cwd, option.split("=", 1)[1]))
            args = args[1:]
        elif option in {"-i", "--ignore-environment"}:
            context = clear(context)
            args = args[1:]
        elif option in {"-0", "--null", "-v", "--debug"}:
            args = args[1:]
        else:
            return None
    return args, context


def options(wrapper: str, args: list[str]) -> tuple[list[str], dict[str, str]] | None:
    grammar = WRAPPER_GRAMMAR[wrapper]
    values = {}
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
