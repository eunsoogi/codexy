"""A smaller new budget must not invalidate an already accepted checkpoint."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = (
    Path(__file__).resolve().parents[3] / "plugins/codexy/skills/"
    "self-improvement/scripts"
)
sys.path.insert(0, str(SCRIPTS))
import wiki_store
from test_continuation_brief import sample


class BriefBudgetTransitionTest(unittest.TestCase):
    def briefs(self):
        previous = sample()
        previous["verified_progress"] = ["verbose prior observation " * 160]
        current = sample()
        return previous, current

    def test_cli_validates_prior_separately_from_smaller_new_budget(self):
        previous, current = self.briefs()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            prior_file, current_file = root / "previous.json", root / "current.json"
            prior_file.write_text(json.dumps(previous))
            current_file.write_text(json.dumps(current))
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "validate_continuation_brief.py"),
                    str(current_file),
                    "--previous",
                    str(prior_file),
                    "--budget-bytes",
                    "1024",
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_store_accepts_a_shorter_valid_checkpoint_without_losing_intent(self):
        previous, current = self.briefs()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            (root / "raw").mkdir()
            (root / "wiki").mkdir()
            for name in ("_index.md", "config.md", "log.md"):
                (root / name).write_text("initial\n")
            base = wiki_store.initialize(root)

            def proposal(identifier, brief, budget):
                return {
                    "id": identifier,
                    "sources": ["source@revision"],
                    "changes": {
                        "_index.md": "checkpoint",
                        "wiki/continuation.json": json.dumps(brief),
                    },
                    "operation": "update checkpoint",
                    "checkpoint": identifier,
                    "brief_budget_bytes": budget,
                }

            first = wiki_store.commit(root, base, proposal("old", previous, 16384))
            second = wiki_store.commit(root, first, proposal("new", current, 1024))
            self.assertNotEqual(first, second)
            _, raw = wiki_store.read_one(root, "wiki/continuation.json")
            self.assertEqual(json.loads(raw)["corrections"], previous["corrections"])


if __name__ == "__main__":
    unittest.main()
