"""Typed GitHub mutation admission shared by connector and CLI adapters."""

from __future__ import annotations

from .github_api import forbidden as api_forbidden
from .github_mutation import (
    Mutation,
    MutationKind,
    cli_number,
    form,
    merge as _merge,
    read_command as _read_command,
    target,
)
from .merge import positive_int
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
    operation = tuple(filtered[:2])
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
    if len(operation) != 2:
        return True
    if operation == ("pr", "merge"):
        mutation = _merge(filtered[2:], cwd)
    elif operation[0] in {"issue", "pr"} and operation[1] in {"create", "edit"}:
        kind = (
            MutationKind.ISSUE_CREATE
            if operation == ("issue", "create")
            else MutationKind.ISSUE_UPDATE
            if operation == ("issue", "edit")
            else MutationKind.PR_CREATE
            if operation == ("pr", "create")
            else MutationKind.PR_UPDATE
        )
        mutation = form(kind, filtered[2:], cwd)
    elif operation[0] in {"issue", "pr"} and operation[1] in {"close", "reopen"}:
        kind = (
            MutationKind.ISSUE_UPDATE
            if operation[0] == "issue"
            else MutationKind.PR_UPDATE
        )
        subject = "issue" if operation[0] == "issue" else "pull_request"
        mutation = _state(
            kind,
            filtered[2:],
            f"{subject}.set_state",
            cwd,
            "open" if operation[1] == "reopen" else "closed",
        )
    elif operation[1] == "comment" and operation[0] in {"issue", "pr"}:
        kind = (
            MutationKind.ISSUE_UPDATE
            if operation[0] == "issue"
            else MutationKind.PR_UPDATE
        )
        mutation = _comment(
            kind,
            filtered[2:],
            f"{operation[0] if operation[0] == 'issue' else 'pull_request'}.comment",
            cwd,
        )
    elif operation == ("pr", "review"):
        mutation = _review(filtered[2:], cwd)
    elif operation == ("pr", "ready"):
        mutation = _ready(filtered[2:])
    else:
        return True
    if mutation is None:
        return True
    selector_repository = getattr(mutation.selector, "repository", None)
    if repository is not None and selector_repository is not None:
        if github_identity(repository) != github_identity(selector_repository):
            return True
    selected_repository = selector_repository or repository
    if not (
        default_owned
        if selected_repository is None
        else github_identity(selected_repository) == owned_identity
    ):
        return True
    return not admitted(mutation)


def _state(
    kind: MutationKind,
    args: list[str],
    operation: str,
    cwd: str,
    state: str | None = None,
) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])):
        return None
    rest = args[1:]
    if not rest:
        reason = None
    elif len(rest) == 2 and rest[0] in {"--reason", "-r"}:
        reason = rest[1]
    elif len(rest) == 1 and rest[0].startswith("--reason="):
        reason = rest[0].split("=", 1)[1]
    else:
        return None
    payload = {"state": state or "closed"}
    if reason is not None:
        payload["state_reason"] = reason.replace("-", "_").replace(" ", "_")
    return Mutation(kind, True, int(args[0]), operation=operation, payload=payload)


def _comment(
    kind: MutationKind, args: list[str], operation: str, cwd: str
) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])):
        return None
    body = _body(args[1:], cwd)
    return (
        None
        if body is None
        else Mutation(
            kind, True, int(args[0]), operation=operation, payload={"comment": body}
        )
    )


def _body(args: list[str], cwd: str) -> str | None:
    if len(args) == 1 and args[0].startswith("--body="):
        return args[0].split("=", 1)[1]
    if len(args) != 2:
        return None
    if args[0] in {"--body", "-b"}:
        return args[1]
    return read_text(cwd, args[1]) if args[0] in {"--body-file", "-F"} else None


def _review(args: list[str], cwd: str) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])):
        return None
    actions = {
        "--approve": "APPROVE",
        "--comment": "COMMENT",
        "--request-changes": "REQUEST_CHANGES",
    }
    action, body, commit, body_present, index = None, None, None, False, 1
    while index < len(args):
        token = args[index]
        if token in actions and action is None:
            action, index = actions[token], index + 1
        elif (
            token in {"--body", "-b", "--body-file", "-F"}
            and not body_present
            and index + 1 < len(args)
        ):
            body = (
                read_text(cwd, args[index + 1])
                if token in {"--body-file", "-F"}
                else args[index + 1]
            )
            if body is None:
                return None
            body_present, index = True, index + 2
        elif (
            token in {"--commit", "--commit-id"}
            and commit is None
            and index + 1 < len(args)
        ):
            commit, index = args[index + 1], index + 2
        else:
            return None
    if action is None or action in {"COMMENT", "REQUEST_CHANGES"} and not body_present:
        return None
    payload = {"action": action}
    if body_present:
        payload["body"] = body
    if commit is not None:
        payload["commit_id"] = commit
    return Mutation(
        MutationKind.PR_UPDATE,
        True,
        int(args[0]),
        operation="pull_request.submit_review",
        payload=payload,
    )


def _ready(args: list[str]) -> Mutation | None:
    if not args or not positive_int(cli_number(args[0])) or len(args) > 2:
        return None
    if len(args) == 2 and args[1] != "--undo":
        return None
    transition = "draft" if len(args) == 2 else "ready"
    operation = (
        "pull_request.convert_to_draft"
        if transition == "draft"
        else "pull_request.mark_ready"
    )
    return Mutation(
        MutationKind.PR_UPDATE,
        True,
        int(args[0]),
        operation=operation,
        payload={"transition": transition},
    )
