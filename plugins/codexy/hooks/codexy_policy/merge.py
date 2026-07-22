"""Squash-merge input validation with canonical closing-reference semantics."""

from .titles import pr_title

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
    if not isinstance(title, str) or not title.endswith(f" (#{number})") or not pr_title(title[: -len(f" (#{number})")]):
        return False
    return isinstance(message, str) and _unique_final_reference(message)


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
