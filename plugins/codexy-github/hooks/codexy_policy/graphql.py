"""Fail-closed GraphQL operation classification for GitHub API admission."""

from __future__ import annotations

import re

from .graphql_parser import document, parse_document
from .repository import github_identity

STRING, NUMBER = "<string>", "<number>"
_TOKEN = re.compile(r'''\s+|,+|#[^\r\n]*|"""(?:\\.|(?!""").)*"""|"(?:\\["\\/bfnrt]|\\u[0-9A-Fa-f]{4}|[^"\\\r\n])*"|\.\.\.|[_A-Za-z][_0-9A-Za-z]*|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?(?![_A-Za-z0-9])|[!$&():=@\[\]{|}]''', re.DOTALL)


def mutation(query: str) -> bool | None:
    """Return whether a syntactically complete document defines a mutation."""
    tokens = _tokens(query)
    if not tokens:
        return None
    return document(tokens)


def admitted(
    query: str, transport: dict[str, str], owned: tuple[str, str, str] | None
) -> bool:
    """Return eligibility for one exact GraphQL mutation or any valid query."""
    tokens = _tokens(query)
    parsed = None if tokens is None else parse_document(tokens)
    if parsed is None:
        return False
    mutations = [item for item in parsed if item.kind == "mutation"]
    if not mutations:
        return True
    if len(parsed) != 1 or len(mutations) != 1 or not _transport_owned(transport, owned):
        return False
    roots = mutations[0].selection
    if len(roots) != 1:
        return False
    field = roots[0]
    if field.alias:
        return False
    payload = _object(dict(field.arguments).get("input"))
    return payload is not None and _check(field.name, payload)


def _transport_owned(fields: dict[str, str], owned: tuple[str, str, str] | None) -> bool:
    owner, name = fields.get("owner"), fields.get("name")
    return isinstance(owner, str) and isinstance(name, str) and github_identity(f"{owner}/{name}") == owned


def _object(value: object) -> dict[str, object] | None:
    if not isinstance(value, tuple) or len(value) != 2 or value[0] != "object": return None
    result = {}
    for key, item in value[1]:
        if not isinstance(key, str) or key in result: return None
        result[key] = item
    return result


def _check(name: str, payload: dict[str, object]) -> bool:
    if name == "createIssue": return _create(payload)
    if name == "createPullRequest": return _create_pr(payload)
    if name == "updateIssue": return _update_issue(payload)
    if name == "updatePullRequest": return _update_pr(payload)
    if name == "closeIssue": return _close_issue(payload)
    if name in {"reopenIssue", "closePullRequest", "reopenPullRequest"}:
        return _identity(payload, "issueId" if name == "reopenIssue" else "pullRequestId")
    if name == "addComment":
        return _common(payload, {"subjectId", "body", "clientMutationId"}, {"subjectId", "body"}) and _id(payload, "subjectId") and _literal(payload["body"])
    if name in {"addLabelsToLabelable", "removeLabelsFromLabelable"}:
        return _list_action(payload, "labelableId", "labelIds")
    if name in {"addAssigneesToAssignable", "removeAssigneesFromAssignable"}:
        return _list_action(payload, "assignableId", "assigneeIds")
    if name in {"addPullRequestReview", "submitPullRequestReview"}: return _review(payload)
    if name == "requestReviews": return _reviewers(payload, {"userIds", "teamIds", "botIds"})
    if name == "requestReviewsByLogin": return _reviewers(payload, {"userLogins", "teamSlugs", "botLogins"})
    if name in {"convertPullRequestToDraft", "markPullRequestReadyForReview"}:
        return _identity(payload, "pullRequestId")
    return False


def _common(payload: dict[str, object], allowed: set[str], required: set[str]) -> bool:
    return set(payload) <= allowed and required <= set(payload)


def _id(payload: dict[str, object], key: str) -> bool:
    return _literal(payload.get(key))


def _literal(value: object) -> bool:
    return value == STRING


def _nullable(value: object) -> bool:
    return _literal(value) or value == "null"


def _list(value: object) -> bool:
    return isinstance(value, tuple) and len(value) == 2 and value[0] == "list" and all(item == STRING for item in value[1])


def _optional(payload: dict[str, object], allowed: set[str]) -> bool:
    if set(payload) - allowed: return False
    for key, value in payload.items():
        if key in {"body", "review", "milestoneId"} and not _nullable(value): return False
        if key.endswith("Ids") and not _list(value): return False
        if key in {"draft", "maintainerCanModify", "union"} and value not in {"true", "false"}: return False
        if key in {"headRepositoryId", "clientMutationId"} and not _nullable(value): return False
        if key in {"title", "headRefName", "baseRefName"} and not _nullable(value): return False
        if key == "commitId" and not _nullable(value): return False
    return True


def _create(payload: dict[str, object]) -> bool:
    allowed = {"repositoryId", "title", "body", "assigneeIds", "milestoneId", "labelIds", "clientMutationId"}
    return _common(payload, allowed, {"repositoryId", "title"}) and _id(payload, "repositoryId") and _literal(payload["title"]) and _optional(payload, allowed)


def _create_pr(payload: dict[str, object]) -> bool:
    allowed = {"repositoryId", "title", "headRefName", "baseRefName", "body", "draft", "maintainerCanModify", "headRepositoryId", "clientMutationId"}
    required = {"repositoryId", "title", "headRefName", "baseRefName"}
    return _common(payload, allowed, required) and all(_id(payload, key) for key in required) and _optional(payload, allowed)


def _update_issue(payload: dict[str, object]) -> bool:
    if not _id(payload, "issueId"): return False
    semantic = set(payload) - {"issueId", "clientMutationId"}
    if semantic and semantic <= {"title", "body"}: return all(_nullable(payload[key]) for key in semantic)
    if semantic in ({"labelIds"}, {"assigneeIds"}): return _list(payload[next(iter(semantic))])
    return semantic == {"milestoneId"} and _nullable(payload["milestoneId"])


def _update_pr(payload: dict[str, object]) -> bool:
    allowed = {"pullRequestId", "title", "body", "baseRefName", "maintainerCanModify", "clientMutationId"}
    return _common(payload, allowed, {"pullRequestId"}) and _id(payload, "pullRequestId") and bool(set(payload) - {"pullRequestId", "clientMutationId"}) and _optional(payload, allowed)


def _close_issue(payload: dict[str, object]) -> bool:
    allowed = {"issueId", "stateReason", "duplicateIssueId", "clientMutationId"}
    if not _common(payload, allowed, {"issueId"}) or not _id(payload, "issueId"): return False
    reason = payload.get("stateReason")
    if reason == "null":
        reason = None
    return (_common(payload, allowed, {"issueId", "stateReason", "duplicateIssueId"}) and reason == "DUPLICATE" and _id(payload, "duplicateIssueId")) or (reason in {None, "COMPLETED", "NOT_PLANNED"} and "duplicateIssueId" not in payload)


def _list_action(payload: dict[str, object], target: str, values: str) -> bool:
    return _common(payload, {target, values, "clientMutationId"}, {target, values}) and _id(payload, target) and _list(payload[values])


def _identity(payload: dict[str, object], key: str) -> bool:
    return _common(payload, {key, "clientMutationId"}, {key}) and _id(payload, key)


def _review(payload: dict[str, object]) -> bool:
    allowed = {"pullRequestId", "event", "action", "body", "review", "commitId", "comments", "fileComments", "clientMutationId"}
    if not _common(payload, allowed, {"pullRequestId"}) or not _id(payload, "pullRequestId"): return False
    actions = [key for key in ("event", "action") if key in payload]
    if len(actions) != 1 or payload[actions[0]] not in {"COMMENT", "APPROVE", "REQUEST_CHANGES"}: return False
    if {"body", "review"} <= set(payload) or any(key in payload and not _nullable(payload[key]) for key in {"body", "review"}): return False
    if "commitId" in payload and payload["commitId"] != "null" and not _id(payload, "commitId"): return False
    return all(key not in payload or payload[key] == "null" or _entries(payload[key]) for key in {"comments", "fileComments"})


def _reviewers(payload: dict[str, object], fields: set[str]) -> bool:
    return _common(payload, {"pullRequestId", *fields, "union", "clientMutationId"}, {"pullRequestId"}) and _id(payload, "pullRequestId") and all(payload[key] == "null" or _list(payload[key]) for key in fields if key in payload) and any(key in payload and _list(payload[key]) for key in fields) and ("union" not in payload or payload["union"] in {"true", "false"})


def _entries(value: object) -> bool:
    allowed = {"body", "path", "position", "line", "side", "start_line", "start_side"}
    if not isinstance(value, tuple) or len(value) != 2 or value[0] != "list": return False
    return all((item := _object(entry)) is not None and set(item) <= allowed and _id(item, "body") and _id(item, "path") and all(_entry_value(item, key) for key in set(item) - {"body", "path"}) for entry in value[1])


def _entry_value(item: dict[str, object], key: str) -> bool:
    value = item[key]
    return value == "null" or (key in {"position", "line", "start_line"} and value == NUMBER) or (key in {"side", "start_side"} and value not in {NUMBER, "..."})


def _tokens(query: str) -> list[str] | None:
    tokens, index = [], 0
    for match in _TOKEN.finditer(query):
        if match.start() != index:
            return None
        token = match.group()
        index = match.end()
        if token.isspace() or token == "," or token.startswith("#"):
            continue
        tokens.append(STRING if token[0] == '"' else NUMBER if token[0].isdigit() or token[0] == "-" else token)
    return tokens if index == len(query) else None
