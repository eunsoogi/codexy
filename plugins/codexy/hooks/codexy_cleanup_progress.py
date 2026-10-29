#!/usr/bin/python3
"""Bounded per-turn cleanup confirmation guidance; never perform cleanup."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import time

if sys.version_info < (3, 10):
    raise SystemExit(125)

sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
from codexy_policy.cleanup_progress import ARCHIVE_TOOLS, continuation, observation


def owned(info: os.stat_result) -> bool:
    """Use POSIX ownership where available; Windows uses the host data root."""
    return os.name == "nt" or info.st_uid == os.getuid()


def marker_dir() -> Path | None:
    """Create a bounded marker directory under host-provided plugin data."""
    value = os.environ.get("PLUGIN_DATA", "")
    path = Path(value)
    if not value or not path.is_absolute():
        return None
    try:
        info = path.lstat()
        if not stat.S_ISDIR(info.st_mode) or not owned(info):
            return None
        if os.name != "nt" and info.st_mode & 0o022:
            return None
        child = path / "cleanup-progress"
        child.mkdir(mode=0o700, exist_ok=True)
        child_info = child.lstat()
        if not stat.S_ISDIR(child_info.st_mode) or not owned(child_info):
            return None
        if os.name != "nt" and child_info.st_mode & 0o077:
            return None
    except (OSError, AttributeError):
        return None
    return child


def key(payload: dict[str, object]) -> str | None:
    session, turn = payload.get("session_id"), payload.get("turn_id")
    if not all(isinstance(v, str) and 0 < len(v) <= 256 for v in (session, turn)):
        return None
    return hashlib.sha256(f"{session}\0{turn}".encode()).hexdigest()


def create(path: Path, value: str) -> bool:
    # Exclusive creation makes duplicate event delivery idempotent and refuses
    # symlinks or replacement of a prior event's evidence.
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            stream.write(value)
        return True
    except FileExistsError:
        try:
            info = path.lstat()
            return stat.S_ISREG(info.st_mode) and owned(info)
        except OSError:
            return False
    except OSError:
        return False


def unique_fields(items: list[tuple[str, object]]) -> dict[str, object]:
    """Reject ambiguous event envelopes before recording any observation."""
    result: dict[str, object] = {}
    for key, value in items:
        if key in result:
            raise ValueError("duplicate field")
        result[key] = value
    return result


def prune(root: Path) -> bool:
    """Bound stale markers without following links or touching other plugin data."""
    try:
        entries = (
            list(root.glob("cleanup-*.archive"))
            + list(root.glob("cleanup-*.checked"))
            + list(root.glob("cleanup-*.interrupted"))
        )
        for entry in entries:
            if not re.fullmatch(
                r"cleanup-[0-9a-f]{64}\.(archive|checked|interrupted)", entry.name
            ):
                continue
            info = entry.lstat()
            if (
                stat.S_ISREG(info.st_mode)
                and owned(info)
                and time.time() - info.st_mtime > 86400
            ):
                entry.unlink()
        return len(entries) < 1024
    except (OSError, AttributeError):
        return False


def main() -> dict[str, str]:
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument(
        "--event", choices=("PostToolUse", "Stop", "Interrupt"), required=True
    )
    event = parser.parse_args().event
    try:
        raw = sys.stdin.buffer.read(1024 * 1024 + 1)
        if len(raw) > 1024 * 1024:
            return {}
        data = json.loads(raw, object_pairs_hook=unique_fields)
        if not isinstance(data, dict) or data.get("hook_event_name") != event:
            return {}
        archive_request = (
            event == "PostToolUse"
            and data.get("tool_name") in ARCHIVE_TOOLS
            and isinstance(data.get("tool_input"), dict)
            and data["tool_input"].get("archived") is True
        )
        result = observation(data) if event == "PostToolUse" else None
        if archive_request and not result:
            return {
                "systemMessage": "Cleanup archive outcome is unknown; preserve resources and check the operation result and archived thread identity."
            }
        root, token = marker_dir(), key(data)
        if root is None or token is None:
            if archive_request:
                return {
                    "systemMessage": "Cleanup progress state is unavailable; confirm the archive result and thread identity before further cleanup."
                }
            return {}
        if not prune(root):
            if archive_request:
                return {
                    "systemMessage": "Cleanup progress state is unavailable; confirm the archive result and thread identity before further cleanup."
                }
            return {}
        prefix = root / ("cleanup-" + token)
        archive = Path(str(prefix) + ".archive")
        checked = Path(str(prefix) + ".checked")
        interrupted = Path(str(prefix) + ".interrupted")
        if event == "Interrupt":
            create(interrupted, "interrupted")
        elif event == "PostToolUse":
            if result and result[0] == "list_attempt":
                create(checked, "attempted")
            elif result and result[1]:
                if not create(archive, result[1]):
                    return {
                        "systemMessage": "Cleanup progress state could not be recorded; confirm the archive result and thread identity before further cleanup."
                    }
        elif (
            data.get("stop_hook_active") is False
            and not checked.exists()
            and not interrupted.exists()
        ):
            # Read bounded regular files only; any unknown state stays non-blocking.
            info = archive.lstat()
            if stat.S_ISREG(info.st_mode) and owned(info) and info.st_size <= 128:
                thread_id = archive.read_text(encoding="utf-8")
                if thread_id and observation(
                    {
                        "hook_event_name": "PostToolUse",
                        "tool_name": "mcp__codex_app__set_thread_archived",
                        "tool_input": {"threadId": thread_id, "archived": True},
                        "tool_response": {"content": []},
                    }
                ):
                    return continuation(thread_id)
    except (OSError, UnicodeError, ValueError, TypeError):
        pass
    return {}


if __name__ == "__main__":
    print(json.dumps(main()))
