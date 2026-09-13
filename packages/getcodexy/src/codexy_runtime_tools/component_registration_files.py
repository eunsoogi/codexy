"""Bounded file checks used by component registration health."""

from __future__ import annotations

import os
import shutil
from pathlib import Path

from .component_core_hooks import DEPENDENCIES as CORE_HOOK_DEPENDENCIES
from .component_integrity import MAX_COMPONENT_BYTES, _read_regular, valid_agent_toml
from .component_manifest import load_component_manifest


def _text(path: Path, root: Path) -> str:
    return _read_regular(root, path.relative_to(root), MAX_COMPONENT_BYTES).decode()


def _regular(path: Path, root: Path, needle: str = "") -> bool:
    contents = _read_regular(root, path.relative_to(root), MAX_COMPONENT_BYTES)
    return bool(contents) and (not needle or valid_agent_toml(contents.decode(), path))


def _launcher(path: Path, root: Path) -> bool:
    contents = _text(path, root) if _regular(path, root) else ""
    if path.suffix == ".cmd":
        return contents.lower().startswith("@echo off")
    command = contents.splitlines()[0][2:].split() if contents.startswith("#!") else []
    return (
        bool(command)
        and _executable(path, root)
        and (os.name == "nt" or shutil.which(command[-1]) is not None)
    )


def _skill(plugin: Path, component: str) -> bool:
    required = load_component_manifest().component(component).asset.required_paths
    if component == "core":
        required += CORE_HOOK_DEPENDENCIES
    name = "wiki" if component == "core" else "git-workflow"
    contents = _text(plugin / f"skills/{name}/SKILL.md", plugin)
    return all(_regular(plugin / path, plugin) for path in required) and (
        contents.startswith(f"---\nname: {name}\n") and "\n---\n" in contents
    )


def _executable(path: Path, root: Path) -> bool:
    return _regular(path, root) and (os.name == "nt" or os.access(path, os.X_OK))
