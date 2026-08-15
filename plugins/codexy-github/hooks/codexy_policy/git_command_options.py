"""Git command-line configuration option parsing."""

from __future__ import annotations

from .repository import UrlRewrite


VALUE_OPTIONS = {
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--super-prefix",
    "--config-env",
    "--exec-path",
}


def alias_option(value: str) -> tuple[str, str] | None:
    if "=" not in value:
        return None
    variable, command = value.split("=", 1)
    section, separator, key = variable.partition(".")
    if section.casefold() != "alias" or not separator:
        return None
    canonical = key.casefold()
    return (
        (canonical, command)
        if canonical
        and all(
            part and part.replace("_", "").isalnum() for part in canonical.split(".")
        )
        else None
    )


def url_rewrite(value: str) -> tuple[bool, UrlRewrite | None]:
    variable, separator, prefix = value.partition("=")
    canonical = variable.casefold()
    if not canonical.startswith("url."):
        return False, None
    if not separator or not prefix or any(char in value for char in "\0\r\n"):
        return True, None
    if canonical.endswith(".pushinsteadof"):
        replacement = variable[4 : -len(".pushinsteadof")]
        push_only = True
    elif canonical.endswith(".insteadof"):
        replacement = variable[4 : -len(".insteadof")]
        push_only = False
    else:
        return True, None
    return True, UrlRewrite(prefix, replacement, push_only) if replacement else None


def option_value(option: str, arguments: list[str]) -> tuple[str, str | None]:
    if option in VALUE_OPTIONS:
        return option, arguments[0] if arguments else None
    for name in (
        "--git-dir",
        "--work-tree",
        "--namespace",
        "--super-prefix",
        "--config-env",
        "--exec-path",
    ):
        if option.startswith(name + "="):
            return name, option[len(name) + 1 :]
    for name in ("-C", "-c"):
        if option.startswith(name) and len(option) > len(name):
            return name, option[len(name) :].removeprefix("=")
    return option, None
