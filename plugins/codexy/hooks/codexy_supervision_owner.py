#!/usr/bin/python3
"""Reconcile explicitly observed owners and capacity transitions."""

import argparse
import json
import os
import sqlite3
import sys

if sys.version_info < (3, 10):
    raise SystemExit(125)
sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))

from codexy_policy.supervision_dispatch import Dispatch
from codexy_policy.supervision_observation import _unique
from codexy_policy.supervision_reconciliation import release_failed, resolve_pending
from codexy_policy.supervision_reservations import ReservationError
from codexy_policy.supervision_storage import open_configured_store


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--plugin-data")
    parser.add_argument("--parent", required=True)
    sub = parser.add_subparsers(dest="action", required=True)
    bootstrap = sub.add_parser("bootstrap")
    bootstrap.add_argument("--confirmed-complete", action="store_true", required=True)
    ready = sub.add_parser("resolve-pending")
    for name in ("request", "pending", "child"):
        ready.add_argument(f"--{name}", required=True)
    ready.add_argument("--confirmed-ready", action="store_true", required=True)
    failed = sub.add_parser("release-failed")
    failed.add_argument("--request", required=True)
    failed_identity = failed.add_mutually_exclusive_group(required=True)
    failed_identity.add_argument("--pending")
    failed_identity.add_argument("--no-pending", action="store_true")
    failed.add_argument("--confirmed-failed", action="store_true", required=True)
    adopt = sub.add_parser("adopt")
    for name in ("request", "issue", "model", "thinking", "child"):
        adopt.add_argument(f"--{name}", required=True)
    for action in ("lookup", "resume", "release"):
        entry = sub.add_parser(action)
        entry.add_argument("--child", required=True)
        if action == "resume":
            entry.add_argument("--limit", type=int, default=8)
        if action == "release":
            entry.add_argument("--confirmed", action="store_true", required=True)
    args = parser.parse_args()
    try:
        store = open_configured_store(args.plugin_data)
        if args.action == "bootstrap":
            raw = sys.stdin.buffer.read(1024 * 1024 + 1)
            if len(raw) > 1024 * 1024:
                raise ReservationError("owner roster too large")
            Dispatch(store).bootstrap(
                args.parent, json.loads(raw, object_pairs_hook=_unique)
            )
            result = {"result": "reconciled"}
        elif args.action == "resolve-pending":
            resolve_pending(store, args.parent, args.request, args.pending, args.child)
            result = {"result": "ready"}
        elif args.action == "release-failed":
            release_failed(store, args.parent, args.request, args.pending)
            result = {"result": "released"}
        elif args.action == "adopt":
            store.adopt(
                args.parent,
                args.request,
                args.issue,
                args.model,
                args.thinking,
                args.child,
            )
            result = {"result": "adopted"}
        else:
            row = store.lookup(args.parent, args.child)
            if row is None:
                raise ReservationError("unknown owner")
            if args.action == "release":
                # The caller must have independently verified the termination;
                # an archive request or timeout alone is not that evidence.
                store.release(args.parent, row["request"], confirmed=args.confirmed)
                result = {"result": "released"}
            elif args.action == "resume":
                result = {
                    "result": store.reserve(
                        args.parent,
                        row["request"],
                        row["issue"],
                        row["model"],
                        row["thinking"],
                        args.limit,
                    )
                }
            else:
                result = {"owner": row}
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except (OSError, ValueError, ReservationError, sqlite3.Error):
        print(json.dumps({"error": "Unable to reconcile supervision owner."}))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
