import json
import re
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class PullRequestSelector:
    repository: str | None
    number: int


def pull_request(value: str) -> PullRequestSelector | None:
    if value.isascii() and value.isdigit() and int(value) > 0:
        return PullRequestSelector(None, int(value))
    match = re.fullmatch(
        r"https://github\.com/([^/\s]+)/([^/\s]+)/pull/([1-9][0-9]*)/?", value
    )
    if match is None:
        return None
    return PullRequestSelector(
        f"{match.group(1)}/{match.group(2)}", int(match.group(3))
    )


GRAPH_STRING = "<string>"
GRAPH_STRING_VALUE = "<string>:"
GRAPH_BINDINGS = {
    "repositoryId": ("repository_id", "repositoryId"),
    "issueId": ("issue_id", "issueId"),
    "pullRequestId": ("pull_request_id", "pullRequestId"),
    "subjectId": ("subject_id", "subjectId", "issue_id", "pull_request_id"),
    "labelableId": ("labelable_id", "labelableId"),
    "assignableId": ("assignable_id", "assignableId"),
    "duplicateIssueId": ("duplicate_issue_id", "duplicateIssueId"),
    "headRepositoryId": ("head_repository_id", "headRepositoryId"),
    "milestoneId": ("milestone_id", "milestoneId"),
    "commitId": ("commit_id", "commitId"),
    "labelIds": ("label_ids", "labelIds"),
    "assigneeIds": ("assignee_ids", "assigneeIds"),
    "userIds": ("user_ids", "userIds"),
    "teamIds": ("team_ids", "teamIds"),
    "botIds": ("bot_ids", "botIds"),
    "userLogins": ("user_logins", "userLogins"),
    "teamSlugs": ("team_slugs", "teamSlugs"),
    "botLogins": ("bot_logins", "botLogins"),
    "clientMutationId": ("client_mutation_id", "clientMutationId"),
}


def graph_object(value: object) -> dict[str, object] | None:
    if not isinstance(value, tuple) or len(value) != 2 or value[0] != "object":
        return None
    result: dict[str, object] = {}
    for key, item in value[1]:
        if not isinstance(key, str) or key in result:
            return None
        result[key] = item
    return result


def graph_literal(value: object) -> bool:
    return isinstance(value, str) and (
        value == GRAPH_STRING or value.startswith(GRAPH_STRING_VALUE)
    )


def graph_string(value: object) -> str | None:
    if not isinstance(value, str) or not value.startswith(GRAPH_STRING_VALUE):
        return None
    return value[len(GRAPH_STRING_VALUE) :]


def graph_keys(key: str) -> tuple[str, ...]:
    return GRAPH_BINDINGS.get(key, (key,))


def graph_bound(value: object, transport: dict[str, str], *keys: str) -> bool:
    actual = graph_string(value)
    if actual is None and isinstance(value, tuple) and len(value) == 2:
        variable = value[1] if value[0] == "variable" else None
        actual = transport.get(variable) if isinstance(variable, str) else None
    return actual is not None and any(transport.get(key) == actual for key in keys)


def graph_id(payload: dict[str, object], key: str, transport: dict[str, str]) -> bool:
    value = payload.get(key)
    return graph_bound(value, transport, *graph_keys(key))


def graph_common(
    payload: dict[str, object], allowed: set[str], required: set[str]
) -> bool:
    return set(payload) <= allowed and required <= set(payload)


def graph_nullable(value: object) -> bool:
    return graph_literal(value) or value == "null"


def graph_list(value: object, *, allow_empty: bool = False) -> bool:
    return (
        isinstance(value, tuple)
        and len(value) == 2
        and value[0] == "list"
        and (allow_empty or bool(value[1]))
        and all(graph_literal(item) for item in value[1])
    )


def graph_bound_list(value, transport, *keys, allow_empty=False) -> bool:
    if not graph_list(value, allow_empty=allow_empty):
        return False
    actual = [graph_string(item) for item in value[1]]
    if any(item is None for item in actual):
        return False
    for key in keys:
        raw = transport.get(key)
        if not isinstance(raw, str):
            continue
        try:
            expected: Any = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if (
            isinstance(expected, list)
            and actual == expected
            and all(isinstance(item, str) for item in expected)
        ):
            return True
    return False


def graph_name(token: str) -> bool:
    return (
        token not in {"<string>", "<number>", "..."}
        and not token.startswith("<string>:")
        and token not in "{}()[]:$&!=@|"
    )


def graph_type(tokens: list[str], index: int) -> int | None:
    if index >= len(tokens):
        return None
    if tokens[index] == "[":
        index = graph_type(tokens, index + 1)
        if index is None or index >= len(tokens) or tokens[index] != "]":
            return None
        index += 1
    elif graph_name(tokens[index]):
        index += 1
    else:
        return None
    return index + 1 if index < len(tokens) and tokens[index] == "!" else index


API_TYPED_FIELDS = {"-F", "--field"}
API_FIELDS = {"-f", "--raw-field"} | API_TYPED_FIELDS
API_VALUES = {"--cache", "--hostname", "--jq", "--preview", "--template"}
API_HEADERS = {"-H", "--header"}
API_FLAGS = {"--include", "-i", "--paginate", "--slurp", "--silent", "--verbose"}


class UnsafeQueryFile(Exception):
    pass


def parse_api_args(
    args: list[str], cwd: str, read_file: Any
) -> tuple[str, str, dict[str, str], str | None] | None:
    method, fields, input_file, positionals, index = None, {}, None, [], 0
    while index < len(args):
        token = args[index]
        if token in {"-X", "--method"}:
            if method is not None or index + 1 >= len(args):
                return None
            method, index = args[index + 1].upper(), index + 2
        elif token.startswith(("--method=", "-X=")):
            if method is not None:
                return None
            method, index = token.split("=", 1)[1].upper(), index + 1
        elif token.startswith("-X") and len(token) > 2:
            if method is not None:
                return None
            method, index = token[2:].removeprefix("=").upper(), index + 1
        elif token in API_FIELDS:
            if index + 1 >= len(args) or not _api_field(
                fields,
                args[index + 1],
                cwd if token in API_TYPED_FIELDS else None,
                read_file,
            ):
                return None
            index += 2
        elif any(token.startswith(option + "=") for option in API_FIELDS):
            typed = any(token.startswith(option + "=") for option in API_TYPED_FIELDS)
            if not _api_field(
                fields, token.split("=", 1)[1], cwd if typed else None, read_file
            ):
                return None
            index += 1
        elif token == "--input":
            if input_file is not None or index + 1 >= len(args):
                return None
            input_file, index = args[index + 1], index + 2
        elif token.startswith("--input="):
            if input_file is not None:
                return None
            input_file, index = token.split("=", 1)[1], index + 1
        elif token in API_VALUES:
            if index + 1 >= len(args):
                return None
            index += 2
        elif any(token.startswith(option + "=") for option in API_VALUES):
            index += 1
        elif token in API_HEADERS:
            if index + 1 >= len(args):
                return None
            index += 2
        elif token.startswith(("-H", "--header=")) and len(token) > 2:
            index += 1
        elif token in API_FLAGS:
            index += 1
        elif token.startswith("-"):
            return None
        else:
            positionals.append(token)
            index += 1
    if len(positionals) != 1 or not positionals[0]:
        return None
    return (
        positionals[0],
        method or ("POST" if fields or input_file else "GET"),
        fields,
        input_file,
    )


def _api_field(
    fields: dict[str, str], value: str, typed_cwd: str | None, read_file: Any
) -> bool:
    name, separator, content = value.partition("=")
    if not separator or not name or name in fields:
        return False
    if typed_cwd is not None and name == "query" and content.startswith("@"):
        content = read_file(typed_cwd, content[1:])
        if content is None:
            raise UnsafeQueryFile
    return fields.setdefault(name, content) == content
