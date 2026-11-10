#!/usr/bin/env python3
"""Check the shape and byte budget of a derived continuation brief, read-only."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

DEFAULT_BUDGET = 16384
TEXT_FIELDS = (
    "original_request",
    "scope",
    "owner",
    "permissions",
    "stop_conditions",
    "next_action",
)
LIST_FIELDS = (
    "corrections",
    "verified_progress",
    "artifacts",
    "validation_results",
    "failed_approaches",
    "remaining_conditions",
    "sources",
)


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate field: {key}")
        result[key] = value
    return result


def validate(raw: bytes, budget: int) -> dict[str, object]:
    if budget <= 0 or len(raw) > budget:
        raise ValueError("brief exceeds its positive UTF-8 byte budget")
    value = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object)
    if not isinstance(value, dict):
        raise ValueError("brief must be an object")
    for field in TEXT_FIELDS:
        if not isinstance(value.get(field), str) or not value[field].strip():
            raise ValueError(f"missing or empty required field: {field}")
    for field in LIST_FIELDS:
        entries = value.get(field)
        if not isinstance(entries, list) or any(
            not isinstance(item, str) or not item.strip() for item in entries
        ):
            raise ValueError(
                f"required field must be a list of nonempty strings: {field}"
            )
    if not value["sources"]:
        raise ValueError("at least one source reference is required")
    return value


def validate_transition(
    previous: dict[str, object], current: dict[str, object]
) -> None:
    """Keep prior intent and unresolved facts unless cited evidence resolves them."""
    if previous["original_request"] != current["original_request"]:
        raise ValueError("the original request changed")
    if not set(previous["corrections"]).issubset(current["corrections"]):
        raise ValueError("a recorded correction is missing")
    for field, resolutions in (
        ("remaining_conditions", "resolved_conditions"),
        ("failed_approaches", "retired_failures"),
    ):
        entries = current.get(resolutions, [])
        if not isinstance(entries, list):
            raise ValueError(f"{resolutions} must be a list")
        resolved = set()
        for entry in entries:
            if (
                not isinstance(entry, dict)
                or set(entry) != {"item", "source"}
                or not isinstance(entry["item"], str)
                or not entry["item"].strip()
                or not isinstance(entry["source"], str)
                or entry["source"] not in current["sources"]
            ):
                raise ValueError(f"{resolutions} needs an item and a cited source")
            resolved.add(entry["item"])
        missing = set(previous[field]) - set(current[field]) - resolved
        if missing:
            raise ValueError(f"prior {field} removed without cited resolution")


def read_bounded(path: Path, budget: int) -> dict[str, object]:
    with path.open("rb") as source:
        return validate(source.read(budget + 1), budget)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("brief", type=Path)
    parser.add_argument("--budget-bytes", type=int, default=DEFAULT_BUDGET)
    parser.add_argument(
        "--previous", type=Path, help="prior valid checkpoint for retention checks"
    )
    args = parser.parse_args()
    try:
        if args.budget_bytes <= 0:
            raise ValueError("brief budget must be positive")
        # Bound the read itself; a huge input must not fill the active context.
        current = read_bounded(args.brief, args.budget_bytes)
        if args.previous is not None:
            # Bound the old read to its own size so a smaller new budget does
            # not erase the evidence needed for the retention comparison.
            prior_budget = max(1, args.previous.stat().st_size)
            validate_transition(read_bounded(args.previous, prior_budget), current)
    except (OSError, UnicodeError, ValueError) as error:
        print(f"brief invalid: {error}", file=sys.stderr)
        raise SystemExit(1) from error
    print("brief structure and byte budget valid; source reconciliation still required")


if __name__ == "__main__":
    main()
