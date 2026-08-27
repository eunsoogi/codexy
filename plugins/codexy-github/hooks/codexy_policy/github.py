"""Typed GitHub mutation admission shared by connector and CLI adapters."""

from __future__ import annotations

from .github_api import forbidden as api_forbidden
from .github_mutation import (
    BodyEvidence,
    BodySource,
    Mutation,
    MutationKind,
    cli_number,
    form,
    target,
)
from .merge import cli as cli_merge, message_valid, positive_int
from .repository import (
    github_identity,
    read_text,
    repository_identity,
    repository_policy_status,
)
from .pull_request import payload_eligible


def admitted(mutation: Mutation) -> bool:
    if not mutation.owned:
        return True
    return payload_eligible(mutation)


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
    if _read_command(filtered):
        return False
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
    elif operation == ["issue", "close"]:
        mutation = _state(MutationKind.ISSUE_UPDATE, filtered[2:], "issue.set_state", cwd)
    elif operation == ["issue", "reopen"]:
        mutation = _state(MutationKind.ISSUE_UPDATE, filtered[2:], "issue.set_state", cwd, "open")
    elif operation == ["issue", "comment"]:
        mutation = _comment(MutationKind.ISSUE_UPDATE, filtered[2:], "issue.comment", cwd)
    elif operation == ["pr", "close"]:
        mutation = _state(MutationKind.PR_UPDATE, filtered[2:], "pull_request.set_state", cwd, "closed")
    elif operation == ["pr", "reopen"]:
        mutation = _state(MutationKind.PR_UPDATE, filtered[2:], "pull_request.set_state", cwd, "open")
    elif operation == ["pr", "comment"]:
        mutation = _comment(MutationKind.PR_UPDATE, filtered[2:], "pull_request.comment", cwd)
    elif operation == ["pr", "review"]:
        mutation = _review(filtered[2:], cwd)
    elif operation == ["pr", "ready"]:
        mutation = _ready(filtered[2:])
    else:
        return True
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
        return True
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


def _state(kind: MutationKind, args: list[str], operation: str, cwd: str, state: str | None = None) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])): return None
    number, rest, reason, index = int(args[0]), args[1:], None, 0
    while index < len(rest):
        if rest[index] in {"--reason", "-r"} and index + 1 < len(rest) and reason is None:
            reason = rest[index + 1].replace("-", "_").replace(" ", "_"); index += 2
        elif rest[index].startswith("--reason=") and reason is None:
            reason = rest[index].split("=", 1)[1].replace("-", "_"); index += 1
        else: return None
    return Mutation(kind, True, number, operation=operation, payload={"state": state or "closed", **({"state_reason": reason} if reason is not None else {})})


def _comment(kind: MutationKind, args: list[str], operation: str, cwd: str) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])): return None
    body, rest, index = None, args[1:], 0
    while index < len(rest):
        if rest[index] in {"--body", "-b"} and index + 1 < len(rest) and body is None:
            body, index = rest[index + 1], index + 2
        elif rest[index].startswith("--body=") and body is None:
            body, index = rest[index].split("=", 1)[1], index + 1
        elif rest[index] in {"--body-file", "-F"} and index + 1 < len(rest) and body is None:
            body, index = read_text(cwd, rest[index + 1]), index + 2
        else: return None
    return None if body is None else Mutation(kind, True, int(args[0]), operation=operation, payload={"comment": body})


def _review(args: list[str], cwd: str) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])): return None
    action, body, commit, rest, index = None, None, None, args[1:], 0
    while index < len(rest):
        token = rest[index]
        if token in {"--approve", "--comment", "--request-changes"} and action is None:
            action = {"--approve": "APPROVE", "--comment": "COMMENT", "--request-changes": "REQUEST_CHANGES"}[token]; index += 1
        elif token in {"--body", "-b", "--body-file", "-F"} and index + 1 < len(rest) and body is None:
            body = read_text(cwd, rest[index + 1]) if token in {"--body-file", "-F"} else rest[index + 1]; index += 2
        elif token in {"--commit", "--commit-id"} and index + 1 < len(rest) and commit is None:
            commit, index = rest[index + 1], index + 2
        else: return None
    if action is None or action in {"COMMENT", "REQUEST_CHANGES"} and body is None: return None
    return Mutation(MutationKind.PR_UPDATE, True, int(args[0]), operation="pull_request.submit_review", payload={"action": action, **({"body": body} if body is not None else {}), **({"commit_id": commit} if commit is not None else {})})


def _ready(args: list[str]) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])) or len(args) > 2 or (len(args) == 2 and args[1] != "--undo"): return None
    return Mutation(MutationKind.PR_UPDATE, True, int(args[0]), operation="pull_request.convert_to_draft" if len(args) == 2 else "pull_request.mark_ready", payload={"transition": "draft" if len(args) == 2 else "ready"})


def _read_command(args: list[str]) -> bool:
    if not args or args[0] in {"--help", "--version", "version", "status", "help"}: return True
    reads = {
        "issue": {"list", "view"}, "label": {"list"}, "pr": {"list", "view", "diff", "checks", "status"},
        "release": {"list", "view"}, "repo": {"list", "view"}, "run": {"list", "view", "watch"},
        "workflow": {"list", "view"}, "auth": {"status"}, "search": {"code", "commits", "issues", "prs", "repos"},
        "project": {"list", "view"},
    }
    return len(args) >= 2 and args[0] in reads and args[1] in reads[args[0]]
