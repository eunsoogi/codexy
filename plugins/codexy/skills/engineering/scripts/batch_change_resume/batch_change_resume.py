#!/usr/bin/env python3
"""Resume an explicit local batch change from durable, validated state."""

from __future__ import annotations

import argparse
import json
import signal
import sys
from pathlib import Path
from threading import Event

sys.dont_write_bytecode = True

from resume import RESUME_SCHEMA, ResumeError, resume_from_path  # noqa: E402
from processes import DEFAULT_OUTPUT_LIMIT_BYTES  # noqa: E402
from state import StateConflict  # noqa: E402


def _arguments(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace-root", required=True)
    parser.add_argument("--input", required=True, help="#1140 input manifest")
    parser.add_argument(
        "--state-root",
        help="batch-local state directory beneath the workspace root",
    )
    parser.add_argument(
        "--results-root",
        help="directory containing validated output artifacts",
    )
    parser.add_argument("--batch-id", help="optional filename-safe batch identity")
    parser.add_argument(
        "--max-output-bytes",
        type=int,
        default=DEFAULT_OUTPUT_LIMIT_BYTES,
    )
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
        result = resume_from_path(
            Path(arguments.workspace_root),
            arguments.input,
            state_root=arguments.state_root,
            results_root=arguments.results_root,
            batch_id=arguments.batch_id,
            output_limit_bytes=arguments.max_output_bytes,
            cancellation_event=cancellation,
        )
    except StateConflict as error:
        print(
            json.dumps(
                {"schema": RESUME_SCHEMA, "status": "conflict", "error": str(error)},
                sort_keys=True,
            )
        )
        return 3
    except (ResumeError, ValueError) as error:
        print(
            json.dumps(
                {"schema": RESUME_SCHEMA, "status": "error", "error": str(error)},
                sort_keys=True,
            )
        )
        return 2
    finally:
        for signal_number, handler in previous.items():
            signal.signal(signal_number, handler)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
