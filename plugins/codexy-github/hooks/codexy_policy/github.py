"""Typed GitHub mutation admission shared by connector and CLI adapters."""

from __future__ import annotations

from .body import has_sections
from .github_api import forbidden as api_forbidden
from .github_mutation import (
    BodyEvidence,
    BodySource,
    Mutation,
    MutationKind,
    form,
    target,
)
from .merge import cli as cli_merge, message_valid, positive_int
from .pull_request import create as pr_create, shell_update
from .repository import (
    github_identity,
    repository_identity,
    repository_policy_status,
)
from .titles import issue_title

ISSUE_SECTIONS = {"## Problem", "## Scope", "## Acceptance Criteria", "## Verification"}


def admitted(mutation: Mutation) -> bool:
    if not mutation.owned:
        return True
    body = mutation.body.text if mutation.body is not None else None
    if mutation.kind == MutationKind.ISSUE_CREATE:
        return issue_title(mutation.title) and has_sections(body, ISSUE_SECTIONS)
    if mutation.kind == MutationKind.ISSUE_UPDATE:
        return (
            mutation.number is not None
            and (mutation.title is None or issue_title(mutation.title))
            and (body is None or has_sections(body, ISSUE_SECTIONS))
        )
    if mutation.kind == MutationKind.PR_CREATE:
        return pr_create(
            {"title": mutation.title, "body": body, "issue": mutation.issue}
        )
    if mutation.kind == MutationKind.PR_UPDATE:
        return shell_update(
            mutation.number, mutation.title, body, mutation.body is not None
        )
    return (
        False
        if mutation.kind == MutationKind.PR_MERGE
        else mutation.merge_method == "squash"
        and message_valid(mutation.number, mutation.title, body)
    )


def forbidden(
    args: list[str],
    cwd: str,
    cwd_owned: bool | None,
    gh_repo_owned: bool | None,
    owned_identity: tuple[str, str, str] | None = None,
    policy_status: bool | None = None,
    policy_bound: bool = False,
) -> bool:
    if not policy_bound and owned_identity is None:
        owned_identity = repository_identity(cwd)
    if not policy_bound:
        policy_status = repository_policy_status(cwd)
    if policy_status is None:
        return True
    selected_target = target(
        args, cwd_owned if gh_repo_owned is None else gh_repo_owned
    )
    if selected_target is None:
        return True
    filtered, default_owned, repository = selected_target
    operation = filtered[:2]
    if filtered[:1] == ["api"]:
        api_owned = (
            default_owned
            if repository is None
            else github_identity(repository) == owned_identity
        )
        return api_forbidden(
            filtered[1:],
            api_owned,
            cwd,
            owned_identity,
            policy_status,
            policy_bound=True,
        )
    if operation == ["pr", "merge"]:
        mutation = _merge(filtered[2:], cwd)
    elif operation == ["pr", "create"]:
        mutation = form(MutationKind.PR_CREATE, filtered[2:], cwd)
    elif operation == ["pr", "edit"]:
        mutation = form(MutationKind.PR_UPDATE, filtered[2:], cwd)
    elif operation == ["issue", "create"]:
        mutation = form(MutationKind.ISSUE_CREATE, filtered[2:], cwd)
    elif operation == ["issue", "edit"]:
        mutation = form(MutationKind.ISSUE_UPDATE, filtered[2:], cwd)
    else:
        return False
    if mutation is None:
        return True
    selector_repository = (
        mutation.selector.repository if mutation.selector is not None else None
    )
    if repository is not None and selector_repository is not None:
        if github_identity(repository) != github_identity(selector_repository):
            return True
    selected_repository = selector_repository or repository
    owned = (
        default_owned
        if selected_repository is None
        else github_identity(selected_repository) == owned_identity
    )
    if not owned:
        return False
    return not admitted(mutation)


def _merge(args: list[str], cwd: str) -> Mutation | None:
    parsed = cli_merge(args, cwd)
    if parsed is None:
        return None
    selector, method, subject, body = parsed
    return Mutation(
        MutationKind.PR_MERGE,
        True,
        selector.number,
        subject,
        BodyEvidence(body, BodySource.INLINE) if body is not None else None,
        merge_method=method,
        selector=selector,
    )
