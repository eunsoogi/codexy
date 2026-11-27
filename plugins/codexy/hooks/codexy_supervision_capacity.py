#!/usr/bin/python3
"""Evaluate correlated supervision events against private plugin state."""

import argparse
import json
import os
import sqlite3
import sys

if sys.version_info < (3, 10):
    raise SystemExit(125)
sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

from codexy_policy.supervision_observation import (
    _unique,
    admit_create_result,
    diagnostic,
    observe_create,
)
from codexy_policy.supervision_reservations import ReservationError
from codexy_policy.supervision_storage import open_store


def denied(outcome: str) -> dict[str, object]:
    detail = diagnostic(outcome)
    return {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": detail["reason"] + ": " + detail["nextAction"],
        }
    }


def main() -> None:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--event", required=True, choices=("PreToolUse", "PostToolUse"))
    event = parser.parse_args().event
    try:
        raw = sys.stdin.buffer.read(1024 * 1024 + 1)
        if len(raw) > 1024 * 1024:
            raise ValueError("oversized hook input")
        payload = json.loads(raw, object_pairs_hook=_unique)
        if not isinstance(payload, dict) or payload.get("hook_event_name") != event:
            raise ValueError("invalid hook input")
        try:
            store = open_store(os.environ.get("PLUGIN_DATA", ""))
        except (OSError, ReservationError, sqlite3.Error):
            if event == "PreToolUse":
                print(json.dumps(denied("store_unavailable"), separators=(",", ":")))
                return
            raise
        if event == "PreToolUse":
            outcome = admit_create_result(store, payload)
            result = {} if outcome == "claimed" else denied(outcome)
        else:
            observation = observe_create(store, payload)
            result = (
                {}
                if observation in ("ready", "pending", "ignored")
                else {
                    "systemMessage": "Creation result is uncertain; retain the reservation and reconcile the actual child or setup identity."
                }
            )
    except (OSError, ValueError, ReservationError, sqlite3.Error):
        result = (
            denied("unmatched_request")
            if event == "PreToolUse"
            else {
                "systemMessage": "Supervision state is unavailable; retain reservations and reconcile before dispatch."
            }
        )
    print(json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    main()
