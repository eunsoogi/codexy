"""Classify a legacy core plugin before any host mutation."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .monolith_baseline import BASELINES, classify_tree


@dataclass(frozen=True)
class MonolithClassification:
    state: str
    version: str | None
    recovery: str


def classify_monolith(root: Path) -> MonolithClassification:
    version = _version(root)
    baseline = BASELINES.get(version) if version else None
    if baseline is None:
        return MonolithClassification(
            "ambiguous", version, "preserve this installation and recover it manually"
        )
    state = classify_tree(root, baseline)
    if state == "supported-unmodified":
        return MonolithClassification(state, version, "run getcodexy migrate")
    if state == "modified":
        return MonolithClassification(
            state, version, "preserve this installation and recover it manually"
        )
    return MonolithClassification(
        "ambiguous", version, "preserve this installation and recover it manually"
    )


def _version(root: Path) -> str | None:
    try:
        value = json.loads(
            (Path(root) / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
        )
    except (OSError, UnicodeDecodeError, ValueError, json.JSONDecodeError):
        return None
    if not isinstance(value, dict) or (value.get("name"), value.get("repository")) != (
        "codexy",
        "https://github.com/eunsoogi/codexy",
    ):
        return None
    return value.get("version") if isinstance(value.get("version"), str) else None
