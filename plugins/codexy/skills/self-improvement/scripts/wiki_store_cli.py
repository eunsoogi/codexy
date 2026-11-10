#!/usr/bin/env python3
"""Initialize, verify, or commit one explicitly authorized Wiki revision."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import wiki_store


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate candidate field: {key}")
        result[key] = value
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("initialize", "verify", "read", "commit"))
    parser.add_argument("root", type=Path, help="absolute, existing Wiki root")
    parser.add_argument("--expected", help="last verified revision for commit")
    parser.add_argument("--candidate", type=Path, help="authorized candidate JSON")
    parser.add_argument("--path", help="root-relative source for bounded read")
    parser.add_argument("--limit", type=int, default=4000, help="read byte limit")
    args = parser.parse_args()
    try:
        if args.operation == "commit":
            if not args.expected or args.candidate is None or args.path is not None:
                raise ValueError("commit requires --expected and --candidate")
            candidate = json.loads(
                args.candidate.read_bytes(), object_pairs_hook=unique_object
            )
            revision = wiki_store.commit(args.root, args.expected, candidate)
        elif args.expected is not None or args.candidate is not None:
            raise ValueError("only commit accepts candidate arguments")
        elif args.operation == "read":
            if not args.path:
                raise ValueError("read requires --path")
            revision, data = wiki_store.read_one(args.root, args.path, args.limit)
            print(
                json.dumps(
                    {
                        "revision": revision,
                        "path": args.path,
                        "content": data.decode("utf-8"),
                    },
                    ensure_ascii=False,
                )
            )
            return
        elif args.path is not None:
            raise ValueError("only read accepts --path")
        elif args.operation == "initialize":
            revision = wiki_store.initialize(args.root)
        else:
            revision, _ = wiki_store.read_current(args.root)
        print(revision)
    except (OSError, UnicodeError, ValueError) as error:
        parser.exit(1, f"Wiki operation failed; preserve prior data: {error}\n")


if __name__ == "__main__":
    main()
