"""Check structural preservation and budget failure without writing memory."""

import importlib.util
import json
import unittest
from pathlib import Path

SCRIPT = (
    Path(__file__).resolve().parents[3] / "plugins/codexy/skills/"
    "self-improvement/scripts/validate_continuation_brief.py"
)
spec = importlib.util.spec_from_file_location("continuation_brief", SCRIPT)
brief = importlib.util.module_from_spec(spec)
spec.loader.exec_module(brief)


def sample():
    return {
        "original_request": "Implement the original request",
        "corrections": ["Keep the latest correction"],
        "scope": "one task",
        "owner": "assigned worker",
        "permissions": "local writes only",
        "stop_conditions": "stop on unknown authority",
        "verified_progress": ["first check passed"],
        "artifacts": ["file at revision A"],
        "validation_results": ["second check failed"],
        "failed_approaches": ["do not retry A"],
        "remaining_conditions": ["repair second check"],
        "next_action": "inspect failure",
        "sources": ["task record at revision A, lines 1-4"],
    }


class ContinuationBriefTest(unittest.TestCase):
    def test_repeated_updates_keep_original_correction_and_failure(self):
        current = sample()
        for n in range(3):
            current["verified_progress"] = [f"latest progress {n}"]
            raw = json.dumps(current, ensure_ascii=False).encode()
            current = brief.validate(raw, 4096)
        self.assertEqual(current["original_request"], "Implement the original request")
        self.assertEqual(current["corrections"], ["Keep the latest correction"])
        self.assertEqual(current["failed_approaches"], ["do not retry A"])

    def test_missing_intent_correction_or_remaining_conditions_fails(self):
        for field in (
            "original_request",
            "corrections",
            "remaining_conditions",
            "sources",
        ):
            data = sample()
            del data[field]
            with self.subTest(field=field), self.assertRaises(ValueError):
                brief.validate(json.dumps(data).encode(), 4096)

    def test_bytes_budget_duplicate_fields_and_missing_sources_fail(self):
        raw = json.dumps(sample(), ensure_ascii=False).encode()
        with self.assertRaises(ValueError):
            brief.validate(raw, len(raw) - 1)
        with self.assertRaises(ValueError):
            brief.validate(b'{"owner":"A","owner":"B"}', 4096)
        data = sample()
        data["sources"] = []
        with self.assertRaises(ValueError):
            brief.validate(json.dumps(data).encode(), 4096)

    def test_transition_rejects_lost_corrections_and_unverified_resolution(self):
        previous = sample()
        current = sample()
        current["corrections"] = []
        with self.assertRaises(ValueError):
            brief.validate_transition(previous, current)
        current = sample()
        current["remaining_conditions"] = []
        with self.assertRaises(ValueError):
            brief.validate_transition(previous, current)
        current["resolved_conditions"] = [
            {"item": "repair second check", "source": "unavailable source"}
        ]
        with self.assertRaises(ValueError):
            brief.validate_transition(previous, current)
        current["resolved_conditions"][0]["source"] = current["sources"][0]
        brief.validate_transition(previous, current)

    def test_transition_keeps_failure_episode_until_cited_retirement(self):
        previous, current = sample(), sample()
        current["failed_approaches"] = []
        with self.assertRaises(ValueError):
            brief.validate_transition(previous, current)
        current["retired_failures"] = [
            {"item": "do not retry A", "source": current["sources"][0]}
        ]
        brief.validate_transition(previous, current)


if __name__ == "__main__":
    unittest.main()
