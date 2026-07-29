#!/usr/bin/env python3
"""Derive a merge authorization only from a fresh GitHub PR capture."""

import argparse
import json
import sys


def fail(message):
    print(f"GitHub merge authorization capture rejected: {message}", file=sys.stderr)
    raise SystemExit(1)


def string(value, field):
    item = value.get(field) if isinstance(value, dict) else None
    return item if isinstance(item, str) and item else None


def parse_state(path):
    try:
        with open(path, encoding="utf-8") as source:
            state = json.load(source)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"could not read GitHub PR state: {error}")
    if not isinstance(state, dict):
        fail("GitHub PR state must be an object")
    return state


def candidate(comment, state):
    author = comment.get("author") if isinstance(comment, dict) else None
    if string(author, "association") not in {"OWNER", "MEMBER"}:
        return None
    number = state["number"]
    base = state["baseRefName"]
    head = state["headRefOid"]
    body = string(comment, "body")
    common = {
        "intent": "merge", "mergeClass": "squash", "prNumber": number,
        "baseRefName": base, "headRefOid": head, "negated": False, "revoked": False,
    }
    if body == f"AUTHORIZE SQUASH MERGE: PR #{number} BASE {base} HEAD {head}":
        return {**common, "kind": "explicit-maintainer-intent", "commentId": string(comment, "id"), "commentUrl": string(comment, "url")}
    if body == f"AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #{number} BASE {base} HEAD {head}":
        return {**common, "kind": "repository-workflow-contract", "contractCommentId": string(comment, "id"), "contractCommentUrl": string(comment, "url"), "target": "current-pull-request"}
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True)
    parser.add_argument("--expected-pr", type=int, required=True)
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--pr-state-file", required=True)
    parser.add_argument("--authorization-file", required=True)
    args = parser.parse_args()
    state = parse_state(args.pr_state_file)
    if state.get("repository") != args.repo:
        fail("repository does not match the requested merge target")
    if state.get("number") != args.expected_pr:
        fail("PR number does not match the requested merge target")
    if string(state, "headRefOid") != args.expected_head:
        fail("PR head does not match the requested merge target")
    if not string(state, "baseRefName"):
        fail("PR base is missing")
    comments = state.get("comments")
    if not isinstance(comments, list):
        fail("PR comments are missing")
    matches = [item for comment in comments if (item := candidate(comment, state))]
    if len(matches) != 1 or not all(matches[0].get(field) for field in ("kind", "prNumber", "baseRefName", "headRefOid")):
        fail("expected exactly one current OWNER or MEMBER authorization comment")
    comment_id = matches[0].get("commentId", matches[0].get("contractCommentId"))
    comment_url = matches[0].get("commentUrl", matches[0].get("contractCommentUrl"))
    prefix = f"https://github.com/{args.repo}/pull/{args.expected_pr}#issuecomment-"
    if not isinstance(comment_id, str) or not isinstance(comment_url, str) or not comment_url.startswith(prefix):
        fail("authorization comment is not an immutable comment on the target PR")
    with open(args.authorization_file, "w", encoding="utf-8") as output:
        json.dump(matches[0], output, separators=(",", ":"))


if __name__ == "__main__":
    main()
