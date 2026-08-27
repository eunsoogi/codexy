"""Fail-closed GraphQL operation classification for GitHub API admission."""

from __future__ import annotations

import json
import re

from .github_target import (
    GRAPH_BINDINGS,
    graph_bound,
    graph_common,
    graph_id,
    graph_literal,
    graph_nullable,
    graph_object,
)
from .graphql_parser import document, parse_document
from .repository import github_identity, read_text
from .repository_issue import graphql_issue
from .repository_pull_request import graphql_review, graphql_reviewers

_STRING_VALUE = "<string>:"
_NUMBER = "<number>"
_TOKEN = re.compile(
    r'''\s+|,+|#[^\r\n]*|"""(?:\\.|(?!""").)*"""|"(?:\\["\\/bfnrt]|\\u[0-9A-Fa-f]{4}|[^"\\\r\n])*"|\.\.\.|[_A-Za-z][_0-9A-Za-z]*|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?(?![_A-Za-z0-9])|[!$&():=@\[\]{|}]''',
    re.DOTALL,
)
BINDING_FIELDS = {"owner", "name", "query"} | {
    field for fields in GRAPH_BINDINGS.values() for field in fields
}
GRAPH_CREATE = {
    "repositoryId",
    "title",
    "headRefName",
    "baseRefName",
    "body",
    "draft",
    "maintainerCanModify",
    "headRepositoryId",
    "clientMutationId",
}
GRAPH_UPDATE = {
    "pullRequestId",
    "title",
    "body",
    "baseRefName",
    "maintainerCanModify",
    "clientMutationId",
}


def mutation(query: str) -> bool | None:
    """Return whether a syntactically complete document defines a mutation."""
    tokens = _tokens(query)
    return None if not tokens else document(tokens)


def admitted(
    query: str, transport: dict[str, str], owned: tuple[str, str, str] | None
) -> bool:
    """Allow queries and exactly one bound, closed-matrix mutation."""
    tokens = _tokens(query)
    parsed = None if tokens is None else parse_document(tokens)
    if parsed is None:
        return False
    mutations = [item for item in parsed if item.kind == "mutation"]
    if not mutations:
        return True
    if (
        len(parsed) != 1
        or len(mutations) != 1
        or not _transport_owned(transport, owned)
    ):
        return False
    roots = mutations[0].selection
    if len(roots) != 1 or roots[0].alias:
        return False
    field = roots[0]
    payload = graph_object(dict(field.arguments).get("input"))
    return (
        payload is not None
        and _client_mutation_bound(payload, transport)
        and (
            graphql_issue(field.name, payload, transport)
            or graphql_pr(field.name, payload, transport)
        )
    )


def input_query(cwd: str, target: str) -> str | None:
    content = read_text(cwd, target)
    if content is None:
        return None
    try:
        body = json.loads(content)
    except json.JSONDecodeError:
        return None
    query = body.get("query") if isinstance(body, dict) else None
    return query if isinstance(query, str) else None


def _transport_owned(
    fields: dict[str, str], owned: tuple[str, str, str] | None
) -> bool:
    owner, name = fields.get("owner"), fields.get("name")
    selected = (
        github_identity(f"{owner}/{name}")
        if isinstance(owner, str) and isinstance(name, str)
        else None
    )
    return (
        selected is not None
        and owned is not None
        and selected == owned
        and not set(fields) - BINDING_FIELDS
    )


def _client_mutation_bound(
    payload: dict[str, object], transport: dict[str, str]
) -> bool:
    value = payload.get("clientMutationId")
    return (
        value is None
        or value == "null"
        or graph_bound(value, transport, "client_mutation_id", "clientMutationId")
    )


def _tokens(query: str) -> list[str] | None:
    tokens: list[str] = []
    index = 0
    for match in _TOKEN.finditer(query):
        if match.start() != index:
            return None
        token = match.group()
        index = match.end()
        if token.isspace() or token == "," or token.startswith("#"):
            continue
        if token.startswith('"'):
            try:
                value = token[3:-3] if token.startswith('"""') else json.loads(token)
            except (TypeError, ValueError):
                return None
            tokens.append(_STRING_VALUE + value)
        else:
            tokens.append(_NUMBER if token[0].isdigit() or token[0] == "-" else token)
    return tokens if index == len(query) else None


def graphql_pr(
    name: str, payload: dict[str, object], transport: dict[str, str]
) -> bool:
    if name == "createPullRequest":
        required = {"repositoryId", "title", "headRefName", "baseRefName"}
        return (
            graph_common(payload, GRAPH_CREATE, required)
            and graph_id(payload, "repositoryId", transport)
            and all(graph_literal(payload[key]) for key in required - {"repositoryId"})
            and _pr_optional(payload, GRAPH_CREATE, transport)
        )
    if name == "updatePullRequest":
        return (
            graph_common(payload, GRAPH_UPDATE, {"pullRequestId"})
            and graph_id(payload, "pullRequestId", transport)
            and bool(set(payload) - {"pullRequestId", "clientMutationId"})
            and _pr_optional(payload, GRAPH_UPDATE, transport)
        )
    if name in {"closePullRequest", "reopenPullRequest"}:
        return _identity(payload, "pullRequestId", transport)
    if name in {"addPullRequestReview", "submitPullRequestReview"}:
        return graphql_review(payload, transport)
    if name == "requestReviews":
        return graphql_reviewers(payload, {"userIds", "teamIds", "botIds"}, transport)
    if name == "requestReviewsByLogin":
        return graphql_reviewers(
            payload, {"userLogins", "teamSlugs", "botLogins"}, transport
        )
    if name in {"convertPullRequestToDraft", "markPullRequestReadyForReview"}:
        return _identity(payload, "pullRequestId", transport)
    return False


def _pr_optional(
    payload: dict[str, object], allowed: set[str], transport: dict[str, str]
) -> bool:
    if set(payload) - allowed:
        return False
    for key, value in payload.items():
        if key in {"title", "body", "baseRefName"} and not graph_nullable(value):
            return False
        if key in {"draft", "maintainerCanModify"} and value not in {"true", "false"}:
            return False
        if (
            key == "headRepositoryId"
            and value != "null"
            and not graph_id(payload, key, transport)
        ):
            return False
        if (
            key == "clientMutationId"
            and value != "null"
            and not graph_bound(
                value, transport, "client_mutation_id", "clientMutationId"
            )
        ):
            return False
    return True


def _identity(payload: dict[str, object], key: str, transport: dict[str, str]) -> bool:
    return graph_common(payload, {key, "clientMutationId"}, {key}) and graph_id(
        payload, key, transport
    )
