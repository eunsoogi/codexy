"""Read the sole tracked package-version authority from the bundled uv lockfile."""

from __future__ import annotations

import re
from importlib.resources import files
from pathlib import Path


_PACKAGE_HEADER = "[[package]]"
_NAME = re.compile(r'^name = "(?P<value>[^"]+)"$')
_VERSION = re.compile(r'^version = "(?P<value>[^"]+)"$')
_SEMVER = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")


def default_package_version() -> str:
    """Return getcodexy's version from its packaged canonical uv.lock."""
    return parse_package_version(_version_lock().read_text(encoding="utf-8"))


def _version_lock() -> Path:
    source_lock = Path(__file__).resolve().parents[2] / "uv.lock"
    if source_lock.is_file():
        return source_lock
    return Path(files("codexy_runtime_tools").joinpath("_version_data/uv.lock"))


def parse_package_version(text: str) -> str:
    """Reject a lockfile without exactly one valid getcodexy package record."""
    records: list[dict[str, str]] = []
    record: dict[str, str] | None = None
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == _PACKAGE_HEADER:
            record = {}
            records.append(record)
            continue
        if stripped.startswith("["):
            record = None
            continue
        if record is None or not stripped or stripped.startswith("#"):
            continue
        for matcher, field in ((_NAME, "name"), (_VERSION, "version")):
            match = matcher.fullmatch(stripped)
            if match:
                if field in record:
                    raise ValueError(
                        f"uv.lock has duplicate {field} in a package record"
                    )
                record[field] = match["value"]
                break

    versions = [
        record.get("version") for record in records if record.get("name") == "getcodexy"
    ]
    if len(versions) != 1 or versions[0] is None or not _SEMVER.fullmatch(versions[0]):
        raise ValueError("uv.lock must contain exactly one getcodexy package version")
    return versions[0]
