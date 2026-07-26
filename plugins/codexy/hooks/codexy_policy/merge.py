"""Squash-merge input validation with canonical closing-reference semantics."""

from .titles import pr_title
from .github_target import PullRequestSelector, pull_request
from .repository import read_text

CLOSING = {"close", "closes", "closed", "fix", "fixes", "fixed", "resolve", "resolves", "resolved"}


def positive_int(value: object) -> bool:
    return type(value) is int and value > 0


def valid_sha(value: object) -> bool:
    return isinstance(value, str) and len(value) == 40 and all(char in "0123456789abcdefABCDEF" for char in value)


def valid(tool_input: dict[str, object]) -> bool:
    number = tool_input.get("pr_number")
    title = tool_input.get("commit_title")
    message = tool_input.get("commit_message")
    if not positive_int(number) or tool_input.get("merge_method") != "squash" or not valid_sha(tool_input.get("expected_head_sha")):
        return False
    return message_valid(number, title, message)


def message_valid(number: object, title: object, message: object) -> bool:
    if not positive_int(number) or not isinstance(title, str) or not title.endswith(f" (#{number})"):
        return False
    return pr_title(title[: -len(f" (#{number})")]) and isinstance(message, str) and _unique_final_reference(message)


def cli(args: list[str], cwd: str) -> tuple[PullRequestSelector, str, str | None, str | None] | None:
    methods, positionals, subject, body, index = [], [], None, None, 0
    while index < len(args):
        if args[index] in {"--squash", "--merge", "--rebase"}:
            methods.append(args[index][2:])
            index += 1
            continue
        matched, value, next_index = _option(args, index, ("--match-head-commit", "--subject", "--body", "--body-file"))
        if matched:
            if value is None or not value:
                return None
            if args[index].startswith("--subject"):
                if subject is not None:
                    return None
                subject = value
            elif args[index].startswith("--body-file"):
                if body is not None or (body := read_text(cwd, value)) is None:
                    return None
            elif args[index].startswith("--body"):
                if body is not None:
                    return None
                body = value
            index = next_index
            continue
        if args[index] == "--delete-branch":
            index += 1
            continue
        if args[index].startswith("-"):
            return None
        positionals.append(args[index])
        index += 1
    if len(methods) != 1 or len(positionals) != 1 or (selector := pull_request(positionals[0])) is None:
        return None
    return selector, methods[0], subject, body


def _option(args: list[str], index: int, options: tuple[str, ...]) -> tuple[bool, str | None, int]:
    for option in options:
        if args[index] == option:
            return True, args[index + 1] if index + 1 < len(args) else None, index + 2
        if args[index].startswith(option + "="):
            return True, args[index].split("=", 1)[1], index + 1
    return False, None, index


def _unique_final_reference(message: str) -> bool:
    lines = [line for line in message.splitlines() if line.strip()]
    final = "" if not lines else lines[-1]
    if not final.startswith("Fixes #") or not _positive_digits(final[7:]):
        return False
    return sum(_closing_count(line) for line in lines) == 1


def _closing_count(line: str) -> int:
    tokens = line.split()
    count = 0
    for index, token in enumerate(tokens):
        if token.removesuffix(":").lower() not in CLOSING:
            continue
        for candidate in tokens[index + 1 :]:
            if _issue_ref(candidate):
                count += 1
            else:
                break
    return count


def _issue_ref(value: str) -> bool:
    value = value.strip(",.")
    if value.startswith("#"):
        return _positive_digits(value[1:])
    if "#" not in value:
        return False
    owner_repo, issue = value.rsplit("#", 1)
    parts = owner_repo.split("/", 1)
    return len(parts) == 2 and all(parts) and _positive_digits(issue) and all(
        char.isascii() and (char.isalnum() or char in "-_ .".replace(" ", "")) for part in parts for char in part
    )


def _positive_digits(value: str) -> bool:
    return value.isascii() and value.isdigit() and int(value) > 0
