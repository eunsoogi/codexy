"""Validated, digest-keyed reviewed-decision input for policy inventory generation."""

import json
from pathlib import Path
from typing import Optional


SCHEMA = "codexy.hooks.review-decisions"
TEST_IDS = {"admission", "inventory", "postcompact", "thread-routing", "topology"}
KEYS = {
    "digest", "text", "event", "input", "decision", "tests", "evidence",
    "positiveTests", "negativeTests", "unavailableEvent", "unavailableInput", "rationale",
}


def load(path: Optional[Path], discovered: list[dict[str, object]]) -> dict[str, dict[str, object]]:
    if path is None:
        return {}
    try:
        payload = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"review decisions file is unreadable: {error}") from error
    if not isinstance(payload, dict) or set(payload) != {"schema", "decisions"}:
        raise ValueError("review decisions file must use the exact schema and decisions keys")
    if payload["schema"] != SCHEMA or not isinstance(payload["decisions"], list):
        raise ValueError("review decisions file has an invalid schema or decisions list")
    known = {item["digest"]: item for item in discovered}
    imported: dict[str, dict[str, object]] = {}
    for decision in payload["decisions"]:
        validated = validate(decision, known)
        digest = validated["digest"]
        if digest in imported:
            raise ValueError(f"review decisions file duplicates digest {digest}")
        imported[digest] = validated
    return imported


def validate(value: object, known: dict[object, dict[str, object]]) -> dict[str, object]:
    if not isinstance(value, dict) or set(value) != KEYS:
        raise ValueError("review decision must use only the supported evidence fields")
    digest = value["digest"]
    found = known.get(digest)
    if not isinstance(digest, str) or found is None:
        raise ValueError(f"review decision names unknown digest {digest!r}")
    if value["text"] != found["text"]:
        raise ValueError(f"review decision has stale text for digest {digest}")
    if value["decision"] != "reviewed-exception" or value["event"] != "unavailable" or value["input"] != "unavailable":
        raise ValueError("review decision must be an unavailable reviewed-exception")
    for name in ("tests", "positiveTests", "negativeTests"):
        entries = value[name]
        if not isinstance(entries, list) or not entries or any(entry not in TEST_IDS for entry in entries):
            raise ValueError(f"review decision has invalid {name}")
    if not isinstance(value["evidence"], list) or not value["evidence"] or not all(isinstance(item, str) and item for item in value["evidence"]):
        raise ValueError("review decision requires nonempty evidence")
    for name in ("unavailableEvent", "unavailableInput", "rationale"):
        if not isinstance(value[name], str) or not value[name].strip():
            raise ValueError(f"review decision requires {name}")
    return value


def with_capability_evidence(prior: dict[str, object], capability: str) -> dict[str, object]:
    item = dict(prior)
    evidence = item.get("evidence", [])
    if not isinstance(evidence, list):
        raise ValueError("review decision evidence must remain a list")
    item["evidence"] = list(dict.fromkeys([capability, *evidence]))
    return item
