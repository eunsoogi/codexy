import json
import os
from pathlib import Path
import re
import stat
import time


MAX_INPUT_BYTES = 1024 * 1024
EVENTS = ("PreToolUse", "Interrupt", "UserPromptSubmit")
UNSUPPORTED_INTERPRETER_EXIT = 125
TRACE_ENV = "CODEXY_WATCHER_TRACE_DIR"
TRACE_FILE_LIMIT = 32
TRACE_RECORD_LIMIT = 4
TRACE_FILE_BYTES = 4096
TRACE_RECORD_BYTES = 512
TRACE_EVENTS = frozenset(
    {"hook_received", "hook_rejected", "runtime_result", "binding_output"}
)
TRACE_FAILURES = frozenset(
    {
        "runtime_lookup_error",
        "runtime_unavailable",
        "spawn_error",
        "timeout",
        "nonzero_exit",
        "invalid_response",
    }
)
TRACE_BOOLEAN_FIELDS = frozenset(
    {
        "eventNameMatches",
        "payloadObject",
        "toolNamePresent",
        "toolNameMatched",
        "toolInputPresent",
        "mainSessionIdPresent",
        "waitSessionIdPresent",
        "parentTokenPresent",
        "turnIdPresent",
        "toolUseIdPresent",
        "runtimeAvailable",
        "bindingPresent",
        "cancellationMatched",
    }
)


class HookTrace:
    """Write bounded, value-free hook observations only to an opted-in private directory."""

    def __init__(self):
        self._fd = _open_trace_slot(os.environ.get(TRACE_ENV))
        self._records = 0
        self._bytes = 0

    def record(self, event, **fields):
        if self._fd is None or event not in TRACE_EVENTS:
            return
        record = {
            "timestampMs": time.time_ns() // 1_000_000,
            "component": "hook",
            "event": event,
        }
        if "eventKind" in fields:
            event_kind = fields["eventKind"]
            if event_kind not in EVENTS:
                return
            record["eventKind"] = event_kind
        for name in TRACE_BOOLEAN_FIELDS:
            value = fields.get(name)
            if type(value) is bool:
                record[name] = value
        if "failureClass" in fields:
            failure = fields["failureClass"]
            if failure not in TRACE_FAILURES:
                return
            record["failureClass"] = failure
        try:
            line = json.dumps(record, separators=(",", ":"), sort_keys=True).encode() + b"\n"
            if (
                self._records >= TRACE_RECORD_LIMIT
                or len(line) > TRACE_RECORD_BYTES
                or self._bytes + len(line) > TRACE_FILE_BYTES
            ):
                self.close()
                return
            written = os.write(self._fd, line)
            if written != len(line):
                self.close()
                return
            self._records += 1
            self._bytes += written
        except (OSError, TypeError, ValueError):
            self.close()

    def close(self):
        if self._fd is None:
            return
        try:
            os.close(self._fd)
        except OSError:
            pass
        self._fd = None


def _open_trace_slot(raw_path):
    # A private pre-created directory and exclusive slot files bound storage without
    # colliding when Codex launches more than one hook or Watcher process.
    if os.name != "posix" or not raw_path:
        return None
    try:
        directory = Path(raw_path)
        if not directory.is_absolute():
            return None
        metadata = directory.lstat()
        if (
            not stat.S_ISDIR(metadata.st_mode)
            or metadata.st_uid != os.getuid()
            or stat.S_IMODE(metadata.st_mode) & 0o077
        ):
            return None
        occupied = set()
        for entry in os.scandir(directory):
            if not re.fullmatch(r"slot-(?:0[0-9]|[12][0-9]|3[01])\.jsonl", entry.name):
                return None
            observed = entry.stat(follow_symlinks=False)
            if (
                not stat.S_ISREG(observed.st_mode)
                or observed.st_uid != os.getuid()
                or stat.S_IMODE(observed.st_mode) & 0o077
                or observed.st_size > TRACE_FILE_BYTES
            ):
                return None
            occupied.add(entry.name)
        for slot in range(TRACE_FILE_LIMIT):
            name = f"slot-{slot:02d}.jsonl"
            if name in occupied:
                continue
            flags = os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_EXCL
            flags |= getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
            try:
                descriptor = os.open(directory / name, flags, 0o600)
            except FileExistsError:
                continue
            observed = os.fstat(descriptor)
            if (
                not stat.S_ISREG(observed.st_mode)
                or observed.st_uid != os.getuid()
                or stat.S_IMODE(observed.st_mode) & 0o077
            ):
                os.close(descriptor)
                try:
                    (directory / name).unlink()
                except OSError:
                    pass
                return None
            return descriptor
    except (OSError, TypeError, ValueError):
        return None
    return None


def handle_input_event(event, payload, invoke):
    if event == "Interrupt":
        # Preserve the complete host payload so runtime cancellation can match its session and turn.
        invoke("--hook-interrupt", payload)
        return
    # Prompt text is irrelevant to cancellation; scope the notification to the originating session.
    session_id = payload.get("session_id")
    if isinstance(session_id, str) and session_id:
        invoke("--hook-user-prompt-submit", {"session_id": session_id})


HOOK_API = (MAX_INPUT_BYTES, EVENTS, UNSUPPORTED_INTERPRETER_EXIT, handle_input_event)
