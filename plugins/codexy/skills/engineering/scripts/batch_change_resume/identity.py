"""Stable identities used to decide whether a saved item may be reused."""

from __future__ import annotations

import os
import platform
import shutil
import sys
from pathlib import Path
from typing import Any, Mapping

from constants import RESUME_SCHEMA
from resume_errors import ResumeError
from runner import RUNNER_SCHEMA
from state import STATE_SCHEMA, StateError, canonical_digest, file_state


def validate_batch_id(value: str) -> str:
    if (
        not isinstance(value, str)
        or not value
        or len(value) > 64
        or value[0] in ".-"
        or any(
            character
            not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
            for character in value
        )
    ):
        raise ValueError("batch id must be a short filename-safe value")
    return value


def source_identity(source: Any) -> str:
    if not isinstance(source, Mapping):
        return canonical_digest({"source": "unknown"})
    return canonical_digest(dict(source))


def stable_batch_id(
    preview: Mapping[str, Any], workspace: Path, explicit: str | None
) -> str:
    if explicit is not None:
        return validate_batch_id(explicit)
    source = preview.get("input")
    if isinstance(source, Mapping) and isinstance(source.get("path"), str):
        stable = {"workspace": str(workspace), "input_path": source["path"]}
    else:
        stable = {
            "workspace": str(workspace),
            "items": [
                {
                    "id": item.get("id"),
                    "original": item.get("original", {}).get("path")
                    if isinstance(item.get("original"), Mapping)
                    else None,
                    "output": item.get("output", {}).get("path")
                    if isinstance(item.get("output"), Mapping)
                    else None,
                }
                for item in preview.get("items", [])
                if isinstance(item, Mapping)
            ],
        }
    return canonical_digest(stable)[:32]


def dependency_fingerprint(token: str, workspace: Path) -> dict[str, Any]:
    candidate: Path | None = None
    if os.path.isabs(token):
        candidate = Path(token)
    else:
        resolved = shutil.which(token, path=os.environ.get("PATH"))
        if resolved is not None:
            candidate = Path(resolved)
        elif token.endswith((".py", ".pyc", ".sh", ".json", ".toml", ".yaml", ".yml")):
            candidate = workspace / token
    if candidate is None:
        return {"token": token, "state": "not-resolved"}
    candidate = candidate.expanduser().absolute()
    if not candidate.exists():
        return {"token": token, "path": str(candidate), "state": {"missing": True}}
    try:
        resolved_candidate = candidate.resolve(strict=True)
        state = file_state(resolved_candidate, "command dependency")
    except StateError as error:
        raise ResumeError(
            f"cannot establish command dependency identity for {candidate}"
        ) from error
    return {"token": token, "path": str(resolved_candidate), "state": state}


def command_dependencies(
    item: Mapping[str, Any], workspace: Path
) -> list[dict[str, Any]]:
    commands = item.get("commands")
    if not isinstance(commands, Mapping):
        return []
    dependencies: list[dict[str, Any]] = []
    for phase in ("transform", "validations"):
        values = commands.get(phase)
        values = [values] if phase == "transform" else values
        if not isinstance(values, list):
            continue
        for command in values:
            if not isinstance(command, Mapping):
                continue
            argv = command.get("argv")
            if not isinstance(argv, list):
                continue
            for position, token in enumerate(argv):
                if not isinstance(token, str):
                    continue
                is_static_path = position == 0 or (
                    os.path.isabs(token)
                    and Path(token).suffix.lower()
                    in {".py", ".pyc", ".sh", ".json", ".toml", ".yaml", ".yml"}
                )
                if is_static_path:
                    dependency = dependency_fingerprint(token, workspace)
                    if dependency not in dependencies:
                        dependencies.append(dependency)
    return dependencies


def environment_identity(item_list: list[Mapping[str, Any]], workspace: Path) -> str:
    executable_tokens: list[str] = []
    for item in item_list:
        commands = item.get("commands")
        if not isinstance(commands, Mapping):
            continue
        command_values = [
            commands.get("transform"),
            *(commands.get("validations") or []),
        ]
        for command in command_values:
            if isinstance(command, Mapping):
                argv = command.get("argv")
                if isinstance(argv, list) and argv and isinstance(argv[0], str):
                    executable_tokens.append(argv[0])
    payload = {
        "environment": sorted(os.environ.items()),
        "os": os.name,
        "platform": platform.platform(),
        "python": sys.executable,
        "executables": [
            dependency_fingerprint(token, workspace) for token in executable_tokens
        ],
    }
    return canonical_digest(payload)


def item_identity(
    item: Mapping[str, Any],
    *,
    workspace: Path,
    results_root: Path,
    environment_identity_value: str,
    output_limit_bytes: int,
) -> str:
    spec = {
        "id": item["id"],
        "workspace": str(workspace),
        "results_root": str(results_root),
        "original_path": item["original"]["path"],
        "output_path": item["output"]["path"],
        "commands": item["commands"],
        "command_dependencies": command_dependencies(item, workspace),
        "environment_identity": environment_identity_value,
        "runner": {"schema": RUNNER_SCHEMA, "output_limit_bytes": output_limit_bytes},
    }
    return canonical_digest(spec)


def operation(
    preview: Mapping[str, Any],
    items: list[Mapping[str, Any]],
    *,
    workspace: Path,
    results_root: Path,
    environment_identity_value: str,
    output_limit_bytes: int,
) -> dict[str, Any]:
    item_identities = {
        str(item["id"]): item_identity(
            item,
            workspace=workspace,
            results_root=results_root,
            environment_identity_value=environment_identity_value,
            output_limit_bytes=output_limit_bytes,
        )
        for item in items
    }
    identity_payload = {
        "workspace": str(workspace),
        "results_root": str(results_root),
        "input_identity": source_identity(preview.get("input")),
        "environment_identity": environment_identity_value,
        "output_limit_bytes": output_limit_bytes,
        "item_identities": item_identities,
    }
    return {
        "schema": RESUME_SCHEMA,
        **identity_payload,
        "identity": canonical_digest(identity_payload),
    }


def new_state(
    batch_id: str,
    items: list[Mapping[str, Any]],
    operation_value: dict[str, Any],
) -> dict[str, Any]:
    input_identity = operation_value["input_identity"]
    return {
        "schema": STATE_SCHEMA,
        "batch_id": batch_id,
        "workspace": operation_value["workspace"],
        "operation": operation_value,
        "items": {
            str(item["id"]): {
                "spec_identity": operation_value["item_identities"][str(item["id"])],
                "input_identity": input_identity,
                "status": "pending",
                "invocations": 0,
                "result": None,
            }
            for item in items
        },
    }
