"""Typed GitHub mutation admission shared by connector and CLI adapters."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any

from .body import has_sections
from .github_api import forbidden as api_forbidden
from .github_target import PullRequestSelector, pull_request
from .merge import cli as cli_merge, message_valid, positive_int
from .pull_request import create as pr_create, shell_update
from .repository import OWNED, github_identity, read_text
from .titles import issue_title

ISSUE_SECTIONS = {"## Problem", "## Scope", "## Acceptance Criteria", "## Verification"}
FORM_VALUES = {
    "issue-create": ("--assignee", "-a", "--label", "-l", "--milestone", "-m", "--project", "-p", "--recover"),
    "issue-update": ("--add-assignee", "--remove-assignee", "--add-label", "--remove-label", "--milestone", "-m", "--add-project", "--remove-project"),
    "pr-create": ("--base", "-B", "--head", "-H", "--assignee", "-a", "--label", "-l", "--milestone", "-m", "--project", "-p", "--reviewer", "-r", "--recover"),
    "pr-update": ("--base", "-B", "--add-assignee", "--remove-assignee", "--add-label", "--remove-label", "--milestone", "-m", "--add-project", "--remove-project", "--add-reviewer", "--remove-reviewer"),
}
FORM_FLAGS = {
    "issue-create": {"--web"}, "issue-update": set(),
    "pr-create": {"--draft", "--maintainer-edit", "--no-maintainer-edit", "--web"}, "pr-update": set(),
}


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


def admitted(mutation: Mutation) -> bool:
    if not mutation.owned:
        return True
    body = mutation.body.text if mutation.body is not None else None
    if mutation.kind == MutationKind.ISSUE_CREATE:
        return issue_title(mutation.title) and has_sections(body, ISSUE_SECTIONS)
    if mutation.kind == MutationKind.ISSUE_UPDATE:
        return mutation.number is not None and (mutation.title is None or issue_title(mutation.title)) and (body is None or has_sections(body, ISSUE_SECTIONS))
    if mutation.kind == MutationKind.PR_CREATE:
        return pr_create({"title": mutation.title, "body": body, "issue": mutation.issue})
    if mutation.kind == MutationKind.PR_UPDATE:
        return shell_update(mutation.number, mutation.title, body, mutation.body is not None)
    return mutation.merge_method == "squash" and message_valid(mutation.number, mutation.title, body)


def connector_admitted(tool: str, data: dict[str, Any]) -> bool:
    operation = tool.rsplit("github_", 1)[-1]
    if operation == "create_issue":
        mutation = _connector(MutationKind.ISSUE_CREATE, data, require_title=True, require_body=True)
    elif operation == "update_issue":
        mutation = _connector(MutationKind.ISSUE_UPDATE, data, number="issue_number")
    elif operation == "create_pull_request":
        mutation = _connector(MutationKind.PR_CREATE, data, require_title=True, require_body=True, issue=True)
    elif operation == "update_pull_request":
        mutation = _connector(MutationKind.PR_UPDATE, data, number="pr_number")
    else:
        return False
    return mutation is not None and admitted(mutation)


def forbidden(args: list[str], cwd: str, cwd_owned: bool | None, gh_repo_owned: bool | None) -> bool:
    target = _target(args, cwd_owned if gh_repo_owned is None else gh_repo_owned)
    if target is None:
        return True
    filtered, default_owned, repository = target
    operation = filtered[:2]
    if filtered[:1] == ["api"]:
        api_owned = default_owned if repository is None else github_identity(repository) == OWNED
        return api_forbidden(filtered[1:], api_owned, cwd)
    if operation == ["pr", "merge"]:
        mutation = _merge(filtered[2:], cwd)
    elif operation == ["pr", "create"]:
        mutation = _form(MutationKind.PR_CREATE, filtered[2:], cwd)
    elif operation == ["pr", "edit"]:
        mutation = _form(MutationKind.PR_UPDATE, filtered[2:], cwd)
    elif operation == ["issue", "create"]:
        mutation = _form(MutationKind.ISSUE_CREATE, filtered[2:], cwd)
    elif operation == ["issue", "edit"]:
        mutation = _form(MutationKind.ISSUE_UPDATE, filtered[2:], cwd)
    else:
        return False
    if mutation is None:
        return True
    selector_repository = mutation.selector.repository if mutation.selector is not None else None
    if repository is not None and selector_repository is not None:
        if github_identity(repository) != github_identity(selector_repository):
            return True
    selected_repository = selector_repository or repository
    owned = default_owned if selected_repository is None else github_identity(selected_repository) == OWNED
    if not owned:
        return False
    return not admitted(mutation)


def _connector(kind: MutationKind, data: dict[str, Any], *, number: str | None = None, require_title: bool = False, require_body: bool = False, issue: bool = False) -> Mutation | None:
    value = data.get(number) if number is not None else None
    if number is not None and not positive_int(value):
        return None
    title = data.get("title")
    body = data.get("body")
    if (require_title or "title" in data) and not isinstance(title, str):
        return None
    if (require_body or "body" in data) and not isinstance(body, str):
        return None
    linked = data.get("issue") if issue else None
    if linked is not None and not positive_int(linked):
        return None
    return Mutation(kind, True, int(value) if positive_int(value) else None, title, BodyEvidence(body, BodySource.INLINE) if isinstance(body, str) else None, int(linked) if positive_int(linked) else None)


def _target(args: list[str], default: bool | None) -> tuple[list[str], bool, str | None] | None:
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
    return filtered, default is not False, repository


def _merge(args: list[str], cwd: str) -> Mutation | None:
    parsed = cli_merge(args, cwd)
    if parsed is None:
        return None
    selector, method, subject, body = parsed
    return Mutation(MutationKind.PR_MERGE, True, selector.number, subject, BodyEvidence(body, BodySource.INLINE) if body is not None else None, merge_method=method, selector=selector)


def _form(kind: MutationKind, args: list[str], cwd: str) -> Mutation | None:
    title, body, body_source, positionals, index = None, None, None, [], 0
    while index < len(args):
        matched, value, next_index = _option(args, index, ("--title", "-t"))
        if matched:
            if title is not None or value is None:
                return None
            title, index = value, next_index
            continue
        matched, value, next_index = _option(args, index, ("--body", "-b"))
        if matched:
            if body_source is not None or value is None:
                return None
            body, body_source, index = value, BodySource.INLINE, next_index
            continue
        matched, value, next_index = _option(args, index, ("--body-file", "-F"))
        if matched:
            if body_source is not None or value is None or (body := read_text(cwd, value)) is None:
                return None
            body_source, index = BodySource.FILE, next_index
            continue
        matched, value, next_index = _option(args, index, FORM_VALUES[kind.value])
        if matched:
            if value is None or not value:
                return None
            index = next_index
            continue
        if args[index] in FORM_FLAGS[kind.value]:
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
        number = None if create or len(positionals) != 1 else _cli_number(positionals[0])
    if (create and positionals) or (not create and number is None):
        return None
    return Mutation(kind, True, number, title, BodyEvidence(body, body_source) if body_source is not None else None, selector=selector)


def _cli_number(value: str) -> int | None:
    return int(value) if value.isascii() and value.isdigit() and int(value) > 0 else None


def _option(args: list[str], index: int, options: tuple[str, ...]) -> tuple[bool, str | None, int]:
    arg = args[index]
    for option in options:
        if arg == option:
            return True, args[index + 1] if index + 1 < len(args) else None, index + 2
        if arg.startswith(option + "="):
            return True, arg.split("=", 1)[1], index + 1
        if len(option) == 2 and arg.startswith(option) and len(arg) > 2:
            return True, arg[2:].removeprefix("="), index + 1
    return False, None, index
