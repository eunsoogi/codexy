"""GitHub CLI policy checks after shell target resolution."""

from __future__ import annotations

from .repository import OWNED, github_identity
from .pull_request import shell_create, shell_update
from .titles import issue_title
from .wrappers import option_value


def forbidden(args: list[str], cwd_owned: bool | None, gh_repo_owned: bool | None) -> bool:
    owned = cwd_owned if gh_repo_owned is None else gh_repo_owned
    filtered, index = [], 0
    while index < len(args):
        arg = args[index]
        if arg in {"-R", "--repo"}:
            if index + 1 >= len(args):
                return owned is not False
            owned = github_identity(args[index + 1]) == OWNED
            index += 2
        elif arg.startswith("--repo="):
            owned = github_identity(arg.split("=", 1)[1]) == OWNED
            index += 1
        else:
            filtered.append(arg)
            index += 1
    if owned is False:
        return False
    operation = filtered[:2]
    if operation == ["pr", "merge"]:
        return any(arg == "--admin" or arg.startswith("--admin=") for arg in filtered[2:])
    if operation == ["pr", "create"]:
        title_present, title = option_value(filtered[2:], ("--title", "-t"))
        body_present, body = option_value(filtered[2:], ("--body", "-b"))
        return not title_present or not body_present or not shell_create(title, body)
    if operation == ["pr", "edit"]:
        number = next((arg for arg in filtered[2:] if arg.isascii() and arg.isdigit()), None)
        title_present, title = option_value(filtered[2:], ("--title", "-t"))
        body_present, body = option_value(filtered[2:], ("--body", "-b"))
        return not shell_update(int(number) if number is not None else None, title if title_present else None, body, body_present)
    if operation in (["issue", "create"], ["issue", "edit"]):
        present, title = option_value(filtered[2:], ("--title", "-t"))
        return (operation[1] == "create" and not present) or (present and not issue_title(title))
    return False
