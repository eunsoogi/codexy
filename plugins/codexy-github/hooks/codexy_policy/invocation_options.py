"""Fail-closed parsers for shell launcher option forms."""

from __future__ import annotations


def command(args: list[str]) -> list[str] | None:
    while args and args[0].startswith("-"):
        if args[0] == "--":
            return args[1:]
        if len(args[0]) < 2 or any(char not in "pVv" for char in args[0][1:]):
            return None
        if "V" in args[0] or "v" in args[0]:
            return []
        args = args[1:]
    return args


def exec_command(args: list[str]) -> list[str] | None:
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


def xargs(args: list[str]) -> list[str] | None:
    values = {
        "-a",
        "--arg-file",
        "-d",
        "--delimiter",
        "-E",
        "--eof",
        "-I",
        "--replace",
        "-L",
        "--max-lines",
        "-n",
        "--max-args",
        "-P",
        "--max-procs",
        "-s",
        "--max-chars",
    }
    flags = {
        "-0",
        "--null",
        "-o",
        "--open-tty",
        "-p",
        "--interactive",
        "-r",
        "--no-run-if-empty",
        "-t",
        "--verbose",
        "-x",
        "--exit",
    }
    while args and args[0].startswith("-"):
        option = args[0]
        if option in {"--help", "--version"}:
            return [] if len(args) == 1 else None
        if option == "--":
            return args[1:]
        if option in values:
            if len(args) < 2:
                return None
            args = args[2:]
        elif (
            option in flags
            or option.startswith(
                tuple(item + "=" for item in values if item.startswith("--"))
            )
            or any(
                option.startswith(item) and len(option) > len(item)
                for item in values
                if len(item) == 2
            )
        ):
            args = args[1:]
        else:
            return None
    return args
