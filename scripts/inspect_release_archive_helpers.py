"""Small helpers shared by the release archive contract entrypoint."""

from importlib import import_module
from pathlib import Path


def print_handoff(root: Path) -> None:
    # Validate the shared manifest before exposing its platform bridge paths to archive comparison.
    validate = import_module("handoff_runtime_contract").validate
    manifest = validate(root / "handoff-runtime.json", root)
    for platform in manifest["platforms"].values():
        print(platform["path"])


def fail_if(condition: bool, message: str) -> None:
    # Keep contract checks fail-fast with the caller's specific diagnostic.
    if condition:
        raise SystemExit(message)
