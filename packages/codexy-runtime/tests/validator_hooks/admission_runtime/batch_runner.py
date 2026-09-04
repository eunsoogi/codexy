from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


def evaluate_record(record: object) -> list[str]:
    if not isinstance(record, dict):
        raise ValueError("batch record must be an object")
    request = record.get("input")
    environment = record.get("environment", {})
    if not isinstance(request, dict) or not isinstance(environment, dict):
        raise ValueError("batch record has invalid input or environment")
    if any(
        not isinstance(key, str) or not isinstance(value, str)
        for key, value in environment.items()
    ):
        raise ValueError("batch environment must contain string key/value pairs")

    from codexy_policy.destructive_command import forbidden as destructive_forbidden
    from codexy_policy.envelope import evaluate
    from codexy_policy.repository_github_command import forbidden as github_forbidden

    original_environment = os.environ.copy()
    try:
        os.environ.update(environment)
        payload = json.dumps(request, separators=(",", ":")).encode("utf-8")
        event = request.get("hook_event_name")
        if not isinstance(event, str):
            raise ValueError("batch input has no event")
        outputs = [
            evaluate(
                event,
                payload,
                frozenset({"Bash"}),
                diagnostic,
                forbidden,
            ).decode("utf-8")
            for diagnostic, forbidden in [
                ("CODEXY_REPOSITORY_GITHUB_COMMAND_", github_forbidden),
                ("CODEXY_DESTRUCTIVE_COMMAND_", destructive_forbidden),
            ]
        ]
        return outputs
    finally:
        os.environ.clear()
        os.environ.update(original_environment)


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--plugin-root", required=True)
    args = parser.parse_args()
    sys.path.insert(0, str(Path(args.plugin_root) / "hooks"))

    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            outputs = evaluate_record(json.loads(line))
        except Exception as error:  # noqa: BLE001 - report a failed batch to Rust.
            print(
                json.dumps({"error": f"{type(error).__name__}: {error}"}),
                flush=True,
            )
            return 1
        print(json.dumps(outputs, separators=(",", ":")), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
