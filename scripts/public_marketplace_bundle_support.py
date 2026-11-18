"""Shared validation for the public marketplace bundle lifecycle check."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
TAG = re.compile(r"v(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
# CI and the installed bootstrap consume the same archive/digest/content checks.
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "packages/getcodexy/src"))
from codexy_runtime_tools.public_marketplace_bundle import (  # noqa: E402, F401
    COMPONENTS,
    LEGACY_UPGRADE_VERSION,
    SHA256,
    safe_extract,
    verify_bundle,
    verify_contents,
)

CONTRACT = Path(".agents/plugins/release-publish-contract.json")


def fail(message: str) -> None:
    raise SystemExit(message)


def run(*arguments: str) -> str:
    # Surface a bounded command prefix with the tool diagnostic instead of treating failed Git/gh data as empty.
    result = subprocess.run(
        arguments,
        check=False,
        text=True,
        capture_output=True,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        fail(f"{' '.join(arguments[:3])} failed: {detail}")
    return result.stdout


def read_json(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"could not read JSON {path}: {error}")
    if not isinstance(value, dict):
        fail(f"JSON root must be an object: {path}")
    return value


def canonical_version(value: object, label: str) -> str:
    if not isinstance(value, str) or not VERSION.fullmatch(value):
        fail(f"{label} must be canonical MAJOR.MINOR.PATCH")
    if any(int(component) > 2_147_483_647 for component in value.split(".")):
        fail(f"{label} exceeds the supported version component bound")
    return value


def contract_tag(contract: dict, label: str) -> tuple[str, str]:
    bootstrap = contract.get("bootstrap")
    runtime = contract.get("runtime")
    if not isinstance(bootstrap, dict) or not isinstance(runtime, dict):
        fail(f"{label} must contain bootstrap and runtime objects")
    version = canonical_version(
        bootstrap.get("selectedVersion"), f"{label} selected version"
    )
    tag = runtime.get("selectedTag")
    if not isinstance(tag, str) or not TAG.fullmatch(tag) or tag != f"v{version}":
        fail(f"{label} selected runtime tag must be v{version}")
    return tag, version


def base_contract(root: Path, sha: str) -> dict:
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        fail("BASE_SHA must be a 40-character lowercase commit SHA")
    result = subprocess.run(
        ["git", "show", f"{sha}:{CONTRACT.as_posix()}"],
        cwd=root,
        check=False,
        text=True,
        capture_output=True,
    )
    if result.returncode:
        fail(
            f"could not read the baseline release contract at {sha}: {result.stderr.strip()}"
        )
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        fail(f"baseline release contract is invalid: {error}")
    if not isinstance(value, dict):
        fail("baseline release contract must be an object")
    return value
