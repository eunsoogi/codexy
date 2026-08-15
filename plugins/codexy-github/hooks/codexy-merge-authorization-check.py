#!/usr/bin/env python3
"""Validate an immutable merge authorization against its GitHub PR state."""

import argparse
import json
import sys


def fail(message):
    print(f"merge authorization rejected: {message}", file=sys.stderr)
    raise SystemExit(1)


def pairs(items):
    value = {}
    for key, item in items:
        if key in value:
            raise ValueError(f"must not repeat {key}")
        value[key] = item
    return value


def read(path, label):
    try:
        with open(path, encoding="utf-8") as source:
            return json.load(source, object_pairs_hook=pairs)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        fail(f"{label} JSON error: {error}")


def string(value, field):
    item = value.get(field) if isinstance(value, dict) else None
    return item if isinstance(item, str) and item.strip() else None


def positive_number(value, field):
    item = value.get(field) if isinstance(value, dict) else None
    return (
        item
        if isinstance(item, int) and not isinstance(item, bool) and item > 0
        else None
    )


def authoritative(comment):
    return string(comment, "authorAssociation") in {"OWNER", "MEMBER"} and string(
        comment.get("author") if isinstance(comment, dict) else None, "login"
    )


def matching_comment(state, identifier, url, body):
    comments = state.get("comments") if isinstance(state, dict) else None
    if not isinstance(comments, list):
        return False
    return (
        sum(
            string(comment, "id") == identifier
            and string(comment, "url") == url
            and authoritative(comment)
            and string(comment, "body") == body
            for comment in comments
        )
        == 1
    )


def current(authorization, state, field, state_field=None):
    state_field = state_field or field
    if field == "prNumber":
        return positive_number(authorization, field) == positive_number(
            state, state_field
        )
    return string(authorization, field) == string(state, state_field)


def valid_comment(state, identifier, url, body):
    number = positive_number(state, "number")
    return (
        identifier
        and url
        and number
        and url.startswith("https://github.com/")
        and f"/pull/{number}#issuecomment-" in url
        and matching_comment(state, identifier, url, body)
    )


def validate(authorization, state):
    if not isinstance(authorization, dict) or not isinstance(state, dict):
        fail("authorization and PR state must be JSON objects")
    if (
        authorization.get("negated") is not False
        or authorization.get("revoked") is not False
    ):
        fail("negated and revoked must be boolean false")
    if (
        string(authorization, "intent") != "merge"
        or string(authorization, "mergeClass") != "squash"
        or not current(authorization, state, "prNumber", "number")
        or not current(authorization, state, "baseRefName")
        or not current(authorization, state, "headRefOid")
    ):
        fail("authorization must match the current squash PR state")
    kind = string(authorization, "kind")
    if kind in {"explicit-user-intent", "explicit-maintainer-intent"}:
        if any(
            field in authorization
            for field in ("actor", "recordIssuer", "sourceReference")
        ):
            fail("authorization intent must cite a PR comment")
        identifier, url = (
            string(authorization, "commentId"),
            string(authorization, "commentUrl"),
        )
        body = "AUTHORIZE SQUASH MERGE: PR #{} BASE {} HEAD {}".format(
            authorization["prNumber"],
            authorization["baseRefName"],
            authorization["headRefOid"],
        )
    elif kind == "repository-workflow-contract":
        if string(authorization, "target") != "current-pull-request":
            fail("authorization target must be current-pull-request")
        identifier, url = (
            string(authorization, "contractCommentId"),
            string(authorization, "contractCommentUrl"),
        )
        body = "AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #{} BASE {} HEAD {}".format(
            authorization["prNumber"],
            authorization["baseRefName"],
            authorization["headRefOid"],
        )
    else:
        fail("authorization kind is not authoritative")
    if not valid_comment(state, identifier, url, body):
        fail("authorization must match one OWNER or MEMBER GitHub PR comment")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--authorization-file", required=True)
    parser.add_argument("--pr-state-file", required=True)
    args = parser.parse_args()
    validate(
        read(args.authorization_file, "merge authorization"),
        read(args.pr_state_file, "merge authorization PR state"),
    )


if __name__ == "__main__":
    main()
