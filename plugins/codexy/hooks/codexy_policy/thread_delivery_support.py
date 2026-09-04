"""Transcript authentication and stable delivery-receipt checks."""

from __future__ import annotations

import json
import os
import re
import stat

from .envelope import _pairs

MAX_TRANSCRIPT = 32 * 1024 * 1024
DELEGATION = re.compile(
    r"<codex_delegation>\s*<source_thread_id>([^<]+)</source_thread_id>"
    r"\s*<input>.*?</input>\s*</codex_delegation>",
    re.DOTALL,
)
TRANSITION_KEY = re.compile(r"\btransition key\s*=\s*([^;\n]+)", re.IGNORECASE)
DELIVERY_KEY = re.compile(
    r"\b(?:event id|transition key|state fingerprint)\s*=\s*([^;\n]+)",
    re.IGNORECASE,
)
DELIVERY_TOOLS = frozenset(
    {
        "send_message_to_thread",
        "codex_app__send_message_to_thread",
        "mcp__codex_app__send_message_to_thread",
    }
)
HANDOFF_MARKER = re.compile(
    r"(?im)^[ \t]*(?:terminal(?:\s+(?:parent|child))?|nonterminal(?:\s+wait)?|"
    r"idle(?:\s+wait)?|child|parent|structured|status|compaction)\s+handoff\s*:"
)


def authenticated_parent(records: list[dict], session: str) -> str | None:
    metadata = [item for item in records if item.get("type") == "session_meta"]
    if len(metadata) != 1 or not isinstance(metadata[0].get("payload"), dict):
        raise ValueError("session metadata")
    payload = metadata[0]["payload"]
    if payload.get("id") != session or payload.get("session_id", session) != session:
        raise ValueError("session mismatch")
    context = next(
        (
            index
            for index, item in enumerate(records)
            if item.get("type") == "turn_context"
        ),
        None,
    )
    if context is None:
        raise ValueError("turn context")
    if payload.get("thread_source") == "agent_created_thread":
        return _provenance_parent(records)
    initial = None
    for item in records[context + 1 :]:
        payload = item.get("payload")
        if item.get("type") == "response_item" and isinstance(payload, dict):
            if payload.get("type") == "message" and payload.get("role") == "user":
                initial = payload
                break
    if initial is None or not isinstance(initial.get("content"), list):
        raise ValueError("initial user message")
    parents: list[str] = []
    for part in initial["content"]:
        if not isinstance(part, dict) or part.get("type") != "input_text":
            continue
        text = part.get("text")
        if not isinstance(text, str):
            continue
        parent = delegated_parent(text)
        if parent is not None:
            parents.append(parent)
    if len(parents) > 1 or "" in parents:
        raise ValueError("ambiguous parent")
    return parents[0] if parents else None


def _provenance_parent(records: list[dict]) -> str:
    parents: list[str] = []
    for record in records:
        output = _create_thread_output(record)
        if isinstance(output, str):
            parent = delegated_parent(output)
            if parent is not None:
                parents.append(parent)
    unique = set(parents)
    if not parents or "" in unique or len(unique) != 1:
        raise ValueError("delegation provenance")
    return parents[0]


def _create_thread_output(record: dict) -> object:
    payload = record.get("payload")
    if not isinstance(payload, dict):
        return None
    if record.get("type") == "response_item":
        if (
            payload.get("type") == "function_call_output"
            and payload.get("name") == "create_thread"
            and payload.get("namespace") == "codex_app"
        ):
            return payload.get("output")
        return None
    if record.get("type") != "event_msg" or payload.get("type") != "item_completed":
        return None
    item = payload.get("item")
    if not isinstance(item, dict):
        return None
    if (
        item.get("type") == "FunctionCallOutput"
        and item.get("name") == "create_thread"
        and item.get("namespace") == "codex_app"
    ):
        return item.get("output")
    if (
        item.get("type") == "McpToolCall"
        and item.get("server") == "codex_app"
        and item.get("tool")
        in {"create_thread", "codex_app__create_thread", "mcp__codex_app__create_thread"}
        and item.get("status") == "completed"
    ):
        return item.get("result", item.get("output"))
    return None


def duplicate_delivery(records: list[dict], data: dict) -> bool | str:
    prompt = data.get("prompt")
    if not isinstance(prompt, str):
        return False
    phase = delivery_phase(prompt)
    if phase is None:
        return False
    key = delivery_key(prompt, phase)
    if key is None:
        return "DELIVERY_KEY_REQUIRED"
    recipient = data["threadId"]
    for item in completed_deliveries(records):
        if item.get("threadId") != recipient:
            continue
        prior = item.get("prompt")
        if not isinstance(prior, str) or delivery_phase(prior) != phase:
            continue
        if delivery_key(prior, phase) == key:
            return "DUPLICATE_DELIVERY"
    return False


def delivery_phase(prompt: str) -> str | None:
    lowered = prompt.lower()
    if "post-result receipt" in lowered:
        return "post-result"
    if "pre-delivery" in lowered:
        return "pre-delivery"
    if HANDOFF_MARKER.search(prompt):
        return "handoff"
    if any(
        marker in lowered
        for marker in ("event id=", "transition key=", "state fingerprint=")
    ):
        return "status"
    return None


def delivery_key(prompt: str, phase: str) -> str | None:
    matcher = (
        TRANSITION_KEY if phase in {"pre-delivery", "post-result"} else DELIVERY_KEY
    )
    match = matcher.search(prompt)
    if match is None:
        return None
    key = match.group(1).strip()
    if not key or len(key) > 256 or any(ord(character) < 32 for character in key):
        return None
    return key


def completed_deliveries(records: list[dict]):
    for record in records:
        if record.get("type") != "event_msg":
            continue
        payload = record.get("payload")
        if not isinstance(payload, dict) or payload.get("type") != "item_completed":
            continue
        item = payload.get("item")
        if (
            not isinstance(item, dict)
            or item.get("type") != "McpToolCall"
            or item.get("server") != "codex_app"
            or item.get("tool") not in DELIVERY_TOOLS
            or item.get("status") != "completed"
            or not isinstance(item.get("arguments"), dict)
        ):
            continue
        yield item["arguments"]


def delegated_parent(text: str) -> str | None:
    text = text.strip()
    if not text.startswith("<codex_delegation>"):
        return None
    if text.count("<codex_delegation>") != 1 or text.count("</codex_delegation>") != 1:
        raise ValueError("ambiguous delegation envelope")
    match = DELEGATION.fullmatch(text)
    if match is None:
        raise ValueError("delegation envelope")
    return match.group(1).strip()


def records(path: str) -> list[dict]:
    before = os.lstat(path)
    if not stat.S_ISREG(before.st_mode):
        raise ValueError("transcript type")
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
        | getattr(os, "O_BINARY", 0)
    )
    descriptor = os.open(path, flags)
    try:
        details = os.fstat(descriptor)
        if (
            not stat.S_ISREG(details.st_mode)
            or (before.st_dev, before.st_ino) != (details.st_dev, details.st_ino)
            or details.st_size > MAX_TRANSCRIPT
        ):
            raise ValueError("transcript bounds")
        chunks = []
        remaining = MAX_TRANSCRIPT + 1
        while remaining:
            chunk = os.read(descriptor, min(65536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        raw = b"".join(chunks)
        if len(raw) > MAX_TRANSCRIPT:
            raise ValueError("transcript bounds")
    finally:
        os.close(descriptor)
    parsed = []
    for line in raw.decode("utf-8", "strict").splitlines():
        item = json.loads(line, object_pairs_hook=_pairs)
        if not isinstance(item, dict):
            raise ValueError("transcript record")
        parsed.append(item)
    return parsed
