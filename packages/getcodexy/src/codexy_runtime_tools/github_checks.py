"""Public, installed-package checks for Codexy's generic GitHub workflow."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from .title_policy import issue_title, pr_title

CLOSING = {
    "close",
    "closes",
    "closed",
    "fix",
    "fixes",
    "fixed",
    "resolve",
    "resolves",
    "resolved",
}
REFERENCE = re.compile(r"(?:[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)?#[0-9]+[,.]?$")


def fail(message: str) -> None:
    raise ValueError(message)


def check_issue_title(title: str) -> None:
    if not issue_title(title):
        if (
            title[:1].isascii()
            and title[:1].isupper()
            and not any(
                ord(char) < 32 or ord(char) in {127, 0x85, 0x2028, 0x2029}
                for char in title
            )
        ):
            fail("issue title must not use Conventional Commit style")
        fail("issue title must start with an uppercase descriptive title")


def check_pr_title(title: str) -> None:
    if not pr_title(title):
        fail("PR title must use Conventional Commit style")


def _labels(value: object) -> list[str] | None:
    if isinstance(value, list):
        names = [item.get("name") for item in value if isinstance(item, dict)]
        names.extend(item for item in value if isinstance(item, str))
        return [name for name in names if isinstance(name, str) and name]
    if isinstance(value, dict):
        return _labels(value.get("nodes"))
    return None


def check_pr_labels(path: Path) -> None:
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"could not read PR state: {error}")
    if not isinstance(state, dict) or not isinstance(state.get("state"), str):
        fail("PR state malformed JSON evidence")
    if not any(
        isinstance(state.get(field), str) and state[field]
        for field in (
            "repository",
            "nameWithOwner",
            "headRepository",
            "url",
        )
    ):
        fail("PR state missing repository identity evidence")
    if state["state"].lower() != "open":
        return
    taxonomy = _labels(state.get("repositoryLabels"))
    if taxonomy is None and isinstance(state.get("repository"), dict):
        taxonomy = _labels(state["repository"].get("labels"))
    if taxonomy is None:
        fail("GitHub label evidence missing repositoryLabels taxonomy")
    if taxonomy and not _labels(state.get("labels")):
        fail("PR labels missing label application evidence")


def _references(message: str) -> int:
    count = 0
    for line in message.splitlines():
        words = line.split()
        for index, word in enumerate(words[:-1]):
            if word.rstrip(":").lower() in CLOSING and REFERENCE.fullmatch(
                words[index + 1]
            ):
                count += 1
    return count


def check_merge_message(
    message: str, expected_pr: int, expected_issue: int | None
) -> None:
    subject = message.splitlines()[0] if message.splitlines() else ""
    suffix = f" (#{expected_pr})"
    if not subject.endswith(suffix):
        fail(
            f"merge commit subject must end with the expected PR suffix: (#{expected_pr})"
        )
    if not pr_title(subject.removesuffix(suffix)):
        fail("merge commit subject must use Conventional Commit style")
    references = _references(message)
    if expected_issue is None:
        if references:
            fail("merge commit message must not contain closing references")
        return
    final = next((line for line in reversed(message.splitlines()) if line.strip()), "")
    if references != 1 or final != f"Fixes #{expected_issue}":
        fail(
            "merge commit message must contain exactly one closing reference, and the final closing line must be exactly: Fixes #"
            + str(expected_issue)
        )


def parse() -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="codexy-github-check", allow_abbrev=False)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-issue-title", action="store_true")
    mode.add_argument("--check-pr-title", action="store_true")
    mode.add_argument("--check-pr-labels", action="store_true")
    mode.add_argument("--check-merge-message", action="store_true")
    parser.add_argument("--issue-title")
    parser.add_argument("--pr-title")
    parser.add_argument("--pr-state-file", type=Path)
    parser.add_argument("--expected-pr", type=int)
    parser.add_argument("--expected-issue", type=int)
    parser.add_argument("--merge-message")
    parser.add_argument("--merge-message-file", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse()
    try:
        if args.check_issue_title:
            if args.issue_title is None:
                fail("--issue-title is required")
            check_issue_title(args.issue_title)
        elif args.check_pr_title:
            if args.pr_title is None:
                fail("--pr-title is required")
            check_pr_title(args.pr_title)
        elif args.check_pr_labels:
            if args.pr_state_file is None:
                fail("--pr-state-file is required")
            check_pr_labels(args.pr_state_file)
        else:
            if args.expected_pr is None:
                fail("--expected-pr is required")
            message = args.merge_message
            if args.merge_message_file is not None:
                message = args.merge_message_file.read_text(encoding="utf-8")
            if not message:
                fail("--merge-message or --merge-message-file is required")
            check_merge_message(message, args.expected_pr, args.expected_issue)
    except (OSError, ValueError) as error:
        print(f"codexy GitHub check: {error}", file=sys.stderr)
        return 1
    print("codexy GitHub check ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
