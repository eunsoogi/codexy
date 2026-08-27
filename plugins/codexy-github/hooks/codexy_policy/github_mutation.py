"""Typed GitHub form mutation parsing and evidence models."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any

from .github_target import PullRequestSelector, pull_request
from .repository import read_text


FORM_VALUES = {
    "issue-create": (
        "--assignee",
        "-a",
        "--label",
        "-l",
        "--milestone",
        "-m",
    ),
    "issue-update": (
        "--add-assignee",
        "--remove-assignee",
        "--add-label",
        "--remove-label",
        "--milestone",
        "-m",
    ),
    "pr-create": (
        "--base",
        "-B",
        "--head",
        "-H",
    ),
    "pr-update": (
        "--base",
        "-B",
        "--add-reviewer",
        "--remove-reviewer",
    ),
}
FORM_FLAGS = {
    "issue-create": {"--web"},
    "issue-update": {"--remove-milestone"},
    "pr-create": {"--draft", "--maintainer-edit", "--no-maintainer-edit", "--web"},
    "pr-update": {"--maintainer-edit", "--no-maintainer-edit"},
}
FORM_FIELDS = {
    "issue-create": {
        "--assignee": "assignees", "-a": "assignees",
        "--label": "labels", "-l": "labels",
        "--milestone": "milestone", "-m": "milestone",
    },
    "issue-update": {
        "--add-assignee": "assignees", "--remove-assignee": "assignees",
        "--add-label": "labels", "--remove-label": "labels",
        "--milestone": "milestone", "-m": "milestone",
    },
    "pr-create": {"--base": "base", "-B": "base", "--head": "head", "-H": "head"},
    "pr-update": {
        "--base": "base", "-B": "base",
        "--add-reviewer": "reviewers", "--remove-reviewer": "reviewers",
    },
}
LIST_FIELDS = {"assignees", "labels", "reviewers"}


class MutationKind(Enum):
    ISSUE_CREATE = "issue-create"
    ISSUE_UPDATE = "issue-update"
    PR_CREATE = "pr-create"
    PR_UPDATE = "pr-update"
    PR_MERGE = "pr-merge"


class BodySource(Enum):
    INLINE = "inline"
    FILE = "file"


@dataclass(frozen=True)
class BodyEvidence:
    text: str
    source: BodySource


@dataclass(frozen=True)
class Mutation:
    kind: MutationKind
    owned: bool
    number: int | None = None
    title: str | None = None
    body: BodyEvidence | None = None
    issue: int | None = None
    merge_method: str | None = None
    selector: PullRequestSelector | None = None
    operation: str | None = None
    payload: dict[str, Any] | None = None


def target(
    args: list[str], default: bool | None
) -> tuple[list[str], bool, str | None] | None:
    filtered, repository, index = [], None, 0
    while index < len(args):
        arg = args[index]
        if arg in {"-R", "--repo"}:
            if repository is not None or index + 1 >= len(args):
                return None
            repository, index = args[index + 1], index + 2
        elif arg.startswith("--repo="):
            if repository is not None:
                return None
            repository, index = arg.split("=", 1)[1], index + 1
        elif arg.startswith("-R") and len(arg) > 2:
            if repository is not None:
                return None
            repository, index = arg[2:].removeprefix("="), index + 1
        else:
            filtered.append(arg)
            index += 1
    return filtered, default is True, repository


def form(kind: MutationKind, args: list[str], cwd: str) -> Mutation | None:
    title, body, body_source, positionals, payload, index = None, None, None, [], {}, 0
    while index < len(args):
        matched, value, next_index, _ = option(args, index, ("--title", "-t"))
        if matched:
            if title is not None or value is None:
                return None
            title, index = value, next_index
            payload["title"] = value
            continue
        matched, value, next_index, _ = option(args, index, ("--body", "-b"))
        if matched:
            if body_source is not None or value is None:
                return None
            body, body_source, index = value, BodySource.INLINE, next_index
            payload["body"] = value
            continue
        matched, value, next_index, _ = option(args, index, ("--body-file", "-F"))
        if matched:
            if (
                body_source is not None
                or value is None
                or (body := read_text(cwd, value)) is None
            ):
                return None
            body_source, index = BodySource.FILE, next_index
            payload["body"] = body
            continue
        matched, value, next_index, option_name = option(args, index, FORM_VALUES[kind.value])
        if matched:
            if value is None or not value:
                return None
            if not _put(payload, FORM_FIELDS[kind.value][option_name], value):
                return None
            index = next_index
            continue
        if args[index] in FORM_FLAGS[kind.value]:
            if args[index] == "--remove-milestone":
                if not _put(payload, "milestone", None):
                    return None
            elif args[index] == "--draft":
                payload["draft"] = True
            elif args[index] == "--maintainer-edit":
                payload["maintainer_can_modify"] = True
            elif args[index] == "--no-maintainer-edit":
                payload["maintainer_can_modify"] = False
            index += 1
            continue
        if args[index].startswith("-"):
            return None
        positionals.append(args[index])
        index += 1
    create = kind in {MutationKind.ISSUE_CREATE, MutationKind.PR_CREATE}
    selector = None
    if not create and len(positionals) == 1 and kind == MutationKind.PR_UPDATE:
        selector = pull_request(positionals[0])
        number = selector.number if selector is not None else None
    else:
        number = None if create or len(positionals) != 1 else cli_number(positionals[0])
    if (create and positionals) or (not create and number is None):
        return None
    return Mutation(
        kind,
        True,
        number,
        title,
        BodyEvidence(body, body_source) if body_source is not None else None,
        selector=selector,
        payload=payload,
    )


def cli_number(value: str) -> int | None:
    return (
        int(value) if value.isascii() and value.isdigit() and int(value) > 0 else None
    )


def option(
    args: list[str], index: int, options: tuple[str, ...]
) -> tuple[bool, str | None, int, str | None]:
    arg = args[index]
    for option_name in options:
        if arg == option_name:
            return True, args[index + 1] if index + 1 < len(args) else None, index + 2, option_name
        if arg.startswith(option_name + "="):
            return True, arg.split("=", 1)[1], index + 1, option_name
        if len(option_name) == 2 and arg.startswith(option_name) and len(arg) > 2:
            return True, arg[2:].removeprefix("="), index + 1, option_name
    return False, None, index, None


def _put(payload: dict[str, Any], field: str, value: str) -> bool:
    if field in LIST_FIELDS:
        payload.setdefault(field, []).append(value)
        return True
    if field in payload:
        return False
    payload[field] = value
    return True
