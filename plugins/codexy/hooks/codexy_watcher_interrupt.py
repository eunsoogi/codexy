#!/usr/bin/python3
"""Bind Watcher waits to the host turn and release them on Interrupt."""

import argparse
import hashlib
import json
import os
import platform as host_platform
import re
import stat
import subprocess
import sys
from pathlib import Path

MAX_INPUT_BYTES = 1024 * 1024
EVENTS = ("PreToolUse", "Interrupt")
UNSUPPORTED_INTERPRETER_EXIT = 125
REPOSITORY = "https://github.com/eunsoogi/codexy"
PROTOCOL = "stdio-newline-v1"
SUPPORTED_PLATFORMS = ("darwin-arm64", "linux-x86_64", "windows-x86_64")
SEMVER = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-((?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)

if sys.version_info < (3, 10):
    raise SystemExit(UNSUPPORTED_INTERPRETER_EXIT)


def main() -> int:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument("--event", required=True, choices=EVENTS)
    event = parser.parse_args().event
    raw = sys.stdin.buffer.read(1024 * 1024 + 1)
    if len(raw) > MAX_INPUT_BYTES:
        return 0
    try:
        payload = json.loads(raw)
    except (TypeError, ValueError, json.JSONDecodeError):
        return 0
    if not isinstance(payload, dict) or payload.get("hook_event_name") != event:
        return 0
    if event == "Interrupt":
        _invoke("--hook-interrupt", payload)
        return 0
    tool_name = payload.get("tool_name")
    tool_input = payload.get("tool_input")
    if not isinstance(tool_name, str) or (
        tool_name != "watcher_wait" and not tool_name.endswith("__watcher_wait")
    ):
        return 0
    if not isinstance(tool_input, dict):
        return 0
    result = _invoke("--hook-pretool", payload)
    binding = result.get("requestBinding") if isinstance(result, dict) else None
    if not isinstance(binding, str) or not binding:
        return 0
    updated = dict(tool_input)
    updated["requestBinding"] = binding
    output = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "updatedInput": updated,
        }
    }
    sys.stdout.write(json.dumps(output, separators=(",", ":")))
    return 0


def _invoke(argument: str, payload: dict[str, object]) -> object:
    runtime = _runtime()
    if runtime is None:
        return {}
    try:
        result = subprocess.run(
            [str(runtime), argument],
            input=json.dumps(payload, separators=(",", ":")).encode(),
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=2,
            env=os.environ.copy(),
        )
    except (OSError, subprocess.SubprocessError):
        return {}
    if result.returncode != 0:
        return {}
    try:
        return json.loads(result.stdout)
    except (TypeError, ValueError, json.JSONDecodeError):
        return {}


def _runtime() -> Path | None:
    root = Path(os.environ.get("PLUGIN_ROOT", Path(__file__).resolve().parents[1]))
    name, extension = _runtime_name()
    configured = os.environ.get("CODEXY_RUNTIME_DIR")
    if configured and Path(configured).is_absolute():
        candidate = Path(configured) / f"{name}{extension}"
        if _executable(candidate):
            return candidate
    if _platform_name() not in SUPPORTED_PLATFORMS:
        return None
    candidate = root / "runtime" / f"{name}{extension}"
    return candidate if _executable(candidate) else _cached_runtime(root)


def _runtime_name() -> tuple[str, str]:
    platform = _platform_name()
    return (
        f"codexy-mcp-watcher-{platform}",
        ".exe" if platform == "windows-x86_64" else ".bin",
    )


def _platform_name() -> str:
    override = os.environ.get("CODEXY_RUNTIME_PLATFORM")
    if override:
        return override
    operating_system = {
        "Darwin": "darwin",
        "Linux": "linux",
        "Windows": "windows",
    }.get(host_platform.system(), "unknown")
    architecture = {
        "arm64": "arm64",
        "aarch64": "arm64",
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "AMD64": "x86_64",
    }.get(host_platform.machine(), "unknown")
    return f"{operating_system}-{architecture}"


def _cached_runtime(root: Path) -> Path | None:
    if (
        (root / "runtime-release.json").is_file()
        or any(
            name in os.environ
            for name in (
                "CODEXY_RUNTIME_PACKAGE_PATH",
                "CODEXY_RUNTIME_PACKAGE_URL",
                "CODEXY_RUNTIME_ARTIFACTS_API_URL",
            )
        )
        or os.environ.get("CODEXY_RUNTIME_SOURCE_OVERRIDE")
    ):
        return None
    release = _plugin_release(root / ".codex-plugin/plugin.json")
    if release is None:
        return None
    cache = _cache_root()
    if cache is None:
        return None
    platform = _platform_name()
    runtime = "codexy-mcp-watcher"
    source = "\n".join(
        ("package-default", os.environ.get("CODEXY_RUNTIME_PACKAGE_SHA256", "").lower())
    )
    identity = [
        os.environ.get("CODEXY_RUNTIME_GIT_REPOSITORY", REPOSITORY),
        os.environ.get("CODEXY_RUNTIME_GIT_REF", ""),
        platform,
        PROTOCOL,
        source,
        runtime,
    ]
    digest_input = "\0".join(
        ("codexy.runtime-cache/v2", *identity[:-1], release, runtime)
    )
    key = f"v2-{hashlib.sha256(digest_input.encode()).hexdigest()}"
    extension = ".exe" if platform == "windows-x86_64" else ""
    install_root = cache / key
    installed = install_root / "bin" / f"{runtime}{extension}"
    if not _executable(installed):
        return None
    return installed if _manifest_matches(root, install_root) else None


def _cache_root() -> Path | None:
    configured = os.environ.get("CODEXY_RUNTIME_CACHE_DIR")
    if configured:
        path = Path(configured)
        return path if path.is_absolute() else None
    xdg = os.environ.get("XDG_CACHE_HOME")
    home = os.environ.get("HOME")
    if not xdg and not home:
        return None
    root = Path(xdg) if xdg else Path(home or "") / ".cache"
    return root / "codexy" / "runtime" if root.is_absolute() else None


def _plugin_release(path: Path) -> str | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
        return None
    release = value.get("version") if isinstance(value, dict) else None
    return release if isinstance(release, str) and SEMVER.fullmatch(release) else None


def _manifest_matches(root: Path, install_root: Path) -> bool:
    try:
        expected = json.loads(
            (root / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
        )
        observed = json.loads(
            (install_root / "plugin.json").read_text(encoding="utf-8")
        )
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
        return False
    fields = ("name", "repository", "version")
    if not isinstance(expected, dict) or not isinstance(observed, dict):
        return False
    expected_identity = tuple(expected.get(field) for field in fields)
    observed_identity = tuple(observed.get(field) for field in fields)
    if not all(isinstance(value, str) and value.strip() for value in expected_identity):
        return False
    if not all(isinstance(value, str) and value.strip() for value in observed_identity):
        return False
    if expected_identity[:2] != ("codexy", REPOSITORY):
        return False
    return expected_identity == observed_identity


def _executable(path: Path) -> bool:
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        return False
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    return (
        stat.S_ISREG(metadata.st_mode)
        and not stat.S_ISLNK(metadata.st_mode)
        and not bool(getattr(metadata, "st_file_attributes", 0) & reparse)
        and os.access(path, os.X_OK)
    )


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SystemExit as error:
        if error.code == UNSUPPORTED_INTERPRETER_EXIT:
            raise SystemExit(1) from error
        raise
