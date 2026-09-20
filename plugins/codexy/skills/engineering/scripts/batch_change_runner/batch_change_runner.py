#!/usr/bin/env python3
"""Execute a validated local batch change without replacing originals."""

from __future__ import annotations

import argparse
import json
import signal
import sys
from pathlib import Path
from threading import Event

sys.dont_write_bytecode = True

from processes import DEFAULT_OUTPUT_LIMIT_BYTES  # noqa: E402
from runner import RunnerError, run_from_path  # noqa: E402


SCHEMA = "codexy.batch-change-runner.v1"


def _arguments(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace-root", required=True)
    parser.add_argument("--input", required=True, help="#1140 input manifest")
    parser.add_argument(
        "--results-root",
        help="directory for validated output artifacts (defaults under the workspace)",
    )
    parser.add_argument(
        "--max-output-bytes",
        type=int,
        default=DEFAULT_OUTPUT_LIMIT_BYTES,
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    cancel = Event()

    def request_cancel(signum: int, frame: object) -> None:
        del signum, frame
        cancel.set()

    previous = {
        signal_number: signal.signal(signal_number, request_cancel)
        for signal_number in (signal.SIGINT, signal.SIGTERM)
    }
    try:
        arguments = _arguments(argv)
        workspace = Path(arguments.workspace_root).expanduser().absolute()
        results_root = arguments.results_root or workspace / ".codexy-batch-results"
        result = run_from_path(
            workspace,
            arguments.input,
            results_root=results_root,
            output_limit_bytes=arguments.max_output_bytes,
            cancellation_event=cancel,
        )
    except (RunnerError, ValueError) as error:
        print(json.dumps({"schema": SCHEMA, "status": "error", "error": str(error)}))
        return 2
    finally:
        for signal_number, handler in previous.items():
            signal.signal(signal_number, handler)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
