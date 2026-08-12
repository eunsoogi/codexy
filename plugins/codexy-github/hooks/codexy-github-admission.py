#!/usr/bin/python3
"""Strict captured-title admission for the native Codex GitHub hooks."""

import argparse
import json
import re
import sys

MAX_INPUT = 64 * 1024
CONVENTIONAL = re.compile(r"^[a-z0-9-]+(?:\([a-z0-9_/-]+\))?!?:\s+\S")


def reject_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate object field")
        result[key] = value
    return result


def title_from_stdin():
    payload = sys.stdin.buffer.read(MAX_INPUT + 1)
    if len(payload) > MAX_INPUT:
        return ""
    try:
        value = json.loads(payload, object_pairs_hook=reject_duplicates)
    except (UnicodeDecodeError, ValueError, json.JSONDecodeError):
        return ""
    if not isinstance(value, dict):
        return ""
    tool_input = value.get("tool_input")
    if not isinstance(tool_input, dict):
        return ""
    title = tool_input.get("title")
    return title if isinstance(title, str) else ""


def main():
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--rule", choices=("issue", "pr"), required=True)
    selected = parser.parse_args().rule
    title = title_from_stdin()
    invalid = not title or (
        selected == "issue" and (not title[0].isupper() or CONVENTIONAL.match(title))
    ) or (selected == "pr" and not CONVENTIONAL.match(title))
    if invalid:
        reason = "issue title must be uppercase descriptive prose, not Conventional Commit syntax" if selected == "issue" else "PR title must use Conventional Commit syntax"
        print('{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_GITHUB_ADMISSION: ' + reason + '"}}')
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
