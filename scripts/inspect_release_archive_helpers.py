"""Small helpers shared by the release archive contract entrypoint."""

from importlib import import_module
from pathlib import Path


def print_handoff(root: Path) -> None:
    validate = import_module("handoff_runtime_contract").validate
    manifest = validate(root / "handoff-runtime.json", root)
    for platform in manifest["platforms"].values():
        print(platform["path"])


def fail_if(condition: bool, message: str) -> None:
    if condition:
        raise SystemExit(message)
