#!/usr/bin/env python3
"""Show and apply explicitly selected successful batch-change results."""

from __future__ import annotations

import argparse
import json
import signal
import sys
from pathlib import Path
from threading import Event

sys.dont_write_bytecode = True

from batch_change_apply.errors import ApplyError  # noqa: E402
from batch_change_apply.workflow import APPLY_SCHEMA, apply_from_path  # noqa: E402


def _arguments(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs="?", choices=("apply", "preview"), default="apply")
    parser.add_argument("--workspace-root", required=True)
    parser.add_argument("--results", "--input", dest="results", required=True, help="resume result JSON path, or - for stdin")
    parser.add_argument("--select", nargs="+", action="append", required=True, help="one or more successful item ids to apply")
    parser.add_argument("--state-root", help="batch-local apply state directory beneath the workspace")
    parser.add_argument("--results-root", help="override the validated artifact directory")
    parser.add_argument("--dry-run", action="store_true", help="show selected diffs without replacing files")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    cancellation = Event()

    def request_cancel(signum: int, frame: object) -> None:
        del signum, frame
        cancellation.set()

    previous = {
        signal_number: signal.signal(signal_number, request_cancel)
        for signal_number in (signal.SIGINT, signal.SIGTERM)
    }
    try:
        arguments = _arguments(argv)
        selected = [item_id for group in arguments.select for item_id in group]
        result = apply_from_path(
            Path(arguments.workspace_root),
            arguments.results,
            selected_ids=selected,
            state_root=arguments.state_root,
            results_root=arguments.results_root,
            cancellation_event=cancellation,
            dry_run=arguments.dry_run or arguments.command == "preview",
            entrypoint=str(Path(__file__).resolve()),
        )
    except (ApplyError, ValueError) as error:
        print(json.dumps({"schema": APPLY_SCHEMA, "status": "error", "error": str(error)}, sort_keys=True))
        return 2
    finally:
        for signal_number, handler in previous.items():
            signal.signal(signal_number, handler)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
