"""Bounded option parsing for shell command wrappers."""

from __future__ import annotations


def sudo_command(args: list[str]) -> list[str] | None:
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
        elif option in flag_options or option.startswith(tuple(item + "=" for item in value_options if item.startswith("--"))) or option.startswith("--preserve-env="):
            args = args[1:]
        elif len(option) > 2 and option[:2] in {"-u", "-g", "-h", "-p", "-C", "-D", "-R", "-T"}:
            args = args[1:]
        else:
            return None
    return args


def time_command(args: list[str]) -> list[str] | None:
    value_options = {"-f", "--format", "-o", "--output"}
    flag_options = {"-a", "--append", "-p", "--portability", "-v", "--verbose"}
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "--":
            return args[1:]
        if option in value_options:
            if len(args) < 2:
                return None
            args = args[2:]
        elif option in flag_options or option.startswith(("--format=", "--output=")):
            args = args[1:]
        else:
            return None
    return args


def timeout_command(args: list[str]) -> list[str] | None:
    value_options = {"-k", "--kill-after", "-s", "--signal"}
    flag_options = {"--foreground", "--preserve-status", "-v", "--verbose"}
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "--":
            args = args[1:]
            break
        if option in value_options:
            if len(args) < 2:
                return None
            args = args[2:]
        elif option in flag_options or option.startswith(("--kill-after=", "--signal=")):
            args = args[1:]
        else:
            return None
    return args[1:] if len(args) >= 2 else None


def command_command(args: list[str]) -> list[str] | None:
    while args and args[0].startswith("-"):
        option = args[0]
        if option == "--":
            return args[1:]
        if len(option) < 2 or any(char not in "pVv" for char in option[1:]):
            return None
        if "V" in option or "v" in option:
            return []
        args = args[1:]
    return args


def option_value(args: list[str], options: tuple[str, ...]) -> tuple[bool, str | None]:
    for index, arg in enumerate(args):
        for option in options:
            if arg == option:
                return True, args[index + 1] if index + 1 < len(args) else None
            if arg.startswith(option + "="):
                return True, arg.split("=", 1)[1]
            if len(option) == 2 and arg.startswith(option) and len(arg) > 2:
                return True, arg[2:].removeprefix("=")
    return False, None
