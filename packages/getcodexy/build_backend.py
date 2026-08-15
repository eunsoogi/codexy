"""Materialize getcodexy's canonical lockfile as uv_build package data."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import uv_build


_ROOT = Path(__file__).resolve().parent
_CANONICAL_LOCK = _ROOT / "uv.lock"
_PACKAGED_LOCK = _ROOT / "src/codexy_runtime_tools/_version_data/uv.lock"


def _materialize_version_lock() -> None:
    contents = _CANONICAL_LOCK.read_bytes()
    if not contents:
        raise ValueError("packages/getcodexy/uv.lock must not be empty")
    _PACKAGED_LOCK.parent.mkdir(parents=True, exist_ok=True)
    _PACKAGED_LOCK.write_bytes(contents)


def build_sdist(sdist_directory: str, config_settings: Any = None) -> str:
    _materialize_version_lock()
    return uv_build.build_sdist(sdist_directory, config_settings)


def build_wheel(
    wheel_directory: str,
    config_settings: Any = None,
    metadata_directory: str | None = None,
) -> str:
    _materialize_version_lock()
    return uv_build.build_wheel(wheel_directory, config_settings, metadata_directory)


def build_editable(
    wheel_directory: str,
    config_settings: Any = None,
    metadata_directory: str | None = None,
) -> str:
    _materialize_version_lock()
    return uv_build.build_editable(wheel_directory, config_settings, metadata_directory)


get_requires_for_build_sdist = uv_build.get_requires_for_build_sdist
get_requires_for_build_wheel = uv_build.get_requires_for_build_wheel
get_requires_for_build_editable = uv_build.get_requires_for_build_editable
prepare_metadata_for_build_wheel = uv_build.prepare_metadata_for_build_wheel
prepare_metadata_for_build_editable = uv_build.prepare_metadata_for_build_editable
