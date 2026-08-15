"""Public command line for transactional component lifecycle operations."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

from .component_inspection import doctor, status
from .component_lifecycle import PreAdmissionError, run_operation
from .component_transaction_identity import operation_id
from .component_transition_model import OperationReceipt
from .component_transition_rejections import Rejection, RejectionStage, StateFailure


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="getcodexy", allow_abbrev=False)
    parser.add_argument(
        "--codex",
        type=Path,
        help="optional absolute path supplied by the trusted Codex host",
    )
    parser.add_argument(
        "--codex-home",
        type=Path,
        default=Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")),
    )
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("install", "update", "remove"):
        child = commands.add_parser(command, allow_abbrev=False)
        child.add_argument("components", nargs="*")
        child.add_argument("--json", action="store_true", dest="json_output")
    for command in ("status", "doctor"):
        child = commands.add_parser(command, allow_abbrev=False)
        child.add_argument("--json", action="store_true", dest="json_output")
    bootstrap = commands.add_parser("bootstrap", allow_abbrev=False)
    bootstrap.add_argument("components", nargs="*")
    bootstrap.add_argument("--json", action="store_true", dest="json_output")
    arguments = parser.parse_args(argv)
    try:
        if arguments.command == "status":
            receipt = status(arguments.codex_home, codex=arguments.codex)
        elif arguments.command == "doctor":
            receipt = doctor(arguments.codex_home, codex=arguments.codex)
        else:
            receipt = run_operation(
                arguments.command,
                tuple(getattr(arguments, "components", ())),
                arguments.codex_home,
                arguments.codex,
            )
    except PreAdmissionError as error:
        if arguments.command == "bootstrap" and arguments.json_output:
            print(json.dumps(_bootstrap_host_failure(), sort_keys=True))
            return 2
        print(f"getcodexy {arguments.command}: {error}", file=sys.stderr)
        return 1
    except Exception as error:
        print(f"getcodexy {arguments.command}: {error}", file=sys.stderr)
        return 1
    if arguments.json_output:
        print(json.dumps(receipt, sort_keys=True))
    else:
        print(_human(arguments.command, receipt))
    unhealthy = arguments.command in {"status", "doctor"} and bool(
        receipt.get("errors")
    )
    return 0 if receipt["outcome"] == "completed" and not unhealthy else 2


def _human(command: str, receipt: dict[str, object]) -> str:
    if command == "status":
        return "getcodexy status: installed={installed}; inventory={inventory}; consistency={consistency}; errors={errors}".format(
            installed=",".join(receipt.get("installed_components", [])) or "none",
            inventory=receipt.get("inventory", {}).get("state", "unknown")
            if isinstance(receipt.get("inventory"), dict)
            else "unknown",
            consistency=receipt.get("inventory_consistency", "unknown"),
            errors=",".join(
                error.get("code", "unknown")
                for error in receipt.get("errors", [])
                if isinstance(error, dict)
            )
            or "none",
        )
    if command == "doctor":
        health = receipt.get("component_health", [])
        summary = (
            ",".join(
                f"{entry.get('component')}={entry.get('state')}:{entry.get('repair', 'none')}"
                for entry in health
                if isinstance(entry, dict)
            )
            or "none"
        )
        readiness = receipt.get("host_readiness", {})
        missing = (
            ",".join(readiness.get("missing_requirements", []))
            if isinstance(readiness, dict)
            else "unknown"
        )
        return f"getcodexy doctor: health={summary}; missing={missing or 'none'}; errors={','.join(error.get('code', 'unknown') for error in receipt.get('errors', []) if isinstance(error, dict)) or 'none'}"
    return f"getcodexy {command}: {receipt['outcome']}"


def _bootstrap_host_failure() -> dict[str, object]:
    rejection = Rejection.from_failure(
        RejectionStage.HOST, StateFailure.INCONSISTENT_INSTALLED_STATE
    )
    return OperationReceipt.rejected(
        operation_id(None), "bootstrap", (), (), rejection
    ).encode()


if __name__ == "__main__":
    raise SystemExit(main())
