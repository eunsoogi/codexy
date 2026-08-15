"""Typed, fail-closed effective command resolution for admission policy."""

from __future__ import annotations

from dataclasses import dataclass

from .execution_context import (
    ExecutionContext,
    assigned_variables,
    assignment,
    at,
    expand_tokens,
    export_variables,
    leading_assignments,
    printf_assignment,
    unset_variables,
)
from .executable_identity import (
    available as executable_available,
    resolve as executable_identity,
)
from .invocation_wrappers import (
    environment as wrapper_environment,
    options as wrapper_options,
)
from .invocation_options import (
    command as _command,
    exec_command as _exec,
    xargs as _xargs,
)
from .shell_context import command_option, name, resolve_cwd

MAX_WRAPPER_DEPTH = 8
SHELL_INTERPRETERS = {"sh", "bash", "zsh", "dash"}
OPAQUE_INTERPRETERS = {
    "pwsh",
    "powershell",
    "cmd",
    "python",
    "python3",
    "node",
    "perl",
    "ruby",
}


@dataclass(frozen=True)
class Invocation:
    executable: str | None
    arguments: list[str]
    context: ExecutionContext
    script: str | None = None
    opaque: bool = False
    available: bool = False


def resolve(
    tokens: list[str], context: ExecutionContext, depth: int = 0
) -> Invocation | None:
    """Resolve wrapper launchers and their context before policy classification."""
    if depth > MAX_WRAPPER_DEPTH:
        return None
    return _unwrap(tokens, context, depth)


def _unwrap(
    tokens: list[str], context: ExecutionContext, depth: int
) -> Invocation | None:
    for _ in range(MAX_WRAPPER_DEPTH):
        tokens, context = leading_assignments(tokens, context)
        if not tokens:
            return Invocation(None, [], context, opaque=context.opaque_environment)
        while tokens[:1] == ["!"]:
            tokens = tokens[1:]
            tokens, context = leading_assignments(tokens, context)
        if not tokens:
            return None
        expanded = expand_tokens(tokens, context)
        if expanded is None:
            return Invocation(None, [], context, opaque=True)
        tokens = expanded
        if not tokens:
            return Invocation(None, [], context)
        path = dict(context.environment).get("PATH")
        executable = executable_identity(
            tokens[0], context.cwd, context.executable_aliases, path
        )
        if executable is None:
            return Invocation(None, [], context, opaque=True)
        args = tokens[1:]
        if executable in SHELL_INTERPRETERS | OPAQUE_INTERPRETERS and args == [
            "--version"
        ]:
            return Invocation(
                executable,
                args,
                context,
                available=executable_available(tokens[0], context.cwd, path),
            )
        if executable == "builtin":
            if not args or args[0] != "command":
                return Invocation(None, [], context, opaque=True)
            tokens = args
            continue
        if executable == "export" or executable == "printf" and args[:1] == ["-v"]:
            state = (
                export_variables(args, context)
                if executable == "export"
                else printf_assignment(args, context)
            )
            return None if state is None else Invocation(None, [], state)
        if executable in {"declare", "typeset"}:
            args = args[1:] if args[:1] == ["-x"] and len(args) == 2 else []
        if executable in {"declare", "typeset", "readonly"}:
            declared = assigned_variables(args, context)
            return None if declared is None else Invocation(None, [], declared)
        if executable == "unset":
            remaining = unset_variables(args, context)
            return None if remaining is None else Invocation(None, [], remaining)
        if executable == "env":
            result = wrapper_environment(args, context)
            if result is None:
                return Invocation(None, [], context, opaque=True)
            tokens, context = result
            continue
        if executable in {"nice", "time", "sudo"}:
            result = wrapper_options(executable, args)
            if result is None:
                return Invocation(None, [], context, opaque=True)
            tokens, values = result
            if (
                executable == "sudo"
                and (directory := values.get("-D") or values.get("--chdir")) is not None
            ):
                context = at(context, resolve_cwd(context.cwd, directory))
            continue
        if executable == "timeout":
            result = wrapper_options(executable, args)
            if result is None or len(result[0]) < 2:
                return Invocation(None, [], context, opaque=True)
            tokens = result[0][1:]
            continue
        if executable == "command":
            result = _command(args)
            if result is None:
                return Invocation(None, [], context, opaque=True)
            tokens = result
            continue
        if executable == "exec":
            result = _exec(args)
            if result is None:
                return Invocation(None, [], context, opaque=True)
            tokens = result
            continue
        if executable == "nohup":
            tokens = args[1:] if args[:1] == ["--"] else args
            continue
        if executable == "coproc":
            if not args:
                return None
            tokens = args
            continue
        if executable == "xargs":
            result = _xargs(args)
            if result is None:
                return Invocation(None, [], context, opaque=True)
            if not result:
                return Invocation(None, [], context)
            return Invocation(name(result[0]), result[1:], context, opaque=True)
        if executable in SHELL_INTERPRETERS:
            for index, argument in enumerate(args):
                if command_option(argument):
                    return Invocation(
                        executable,
                        args,
                        context,
                        script=args[index + 1] if index + 1 < len(args) else "",
                    )
            return Invocation(executable, args, context, opaque=True)
        if executable in OPAQUE_INTERPRETERS:
            return Invocation(executable, args, context, opaque=True)
        return Invocation(
            executable,
            args,
            context,
            available=executable_available(tokens[0], context.cwd, path),
        )
    return Invocation(None, [], context, opaque=True)
