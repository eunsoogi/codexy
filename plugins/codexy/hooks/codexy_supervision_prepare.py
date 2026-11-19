#!/usr/bin/python3
"""Reserve capacity for an exact app-creation input supplied on stdin."""

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
from codexy_policy.supervision_reservations import ReservationError
from codexy_policy.supervision_storage import open_store


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--plugin-data", required=True)
    parser.add_argument("--parent", required=True)
    parser.add_argument("--issue", required=True)
    parser.add_argument("--limit", type=int, default=8)
    args = parser.parse_args()
    try:
        raw = sys.stdin.buffer.read(1024 * 1024 + 1)
        if len(raw) > 1024 * 1024:
            raise ValueError("oversized creation input")
        data = json.loads(raw, object_pairs_hook=_unique)
        if not isinstance(data, dict):
            raise ValueError("creation input must be an object")
        result = Dispatch(open_store(args.plugin_data)).prepare(
            args.parent, args.issue, data, args.limit
        )
        print(json.dumps({"result": result}, separators=(",", ":")))
        return 0
    except (OSError, ValueError, ReservationError, sqlite3.Error):
        print(json.dumps({"error": "Unable to prepare supervision reservation."}))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
