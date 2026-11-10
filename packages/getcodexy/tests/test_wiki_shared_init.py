"""Keep shared Wiki initialization isolated and non-destructive."""

import importlib.util
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
spec = importlib.util.spec_from_file_location(
    "wiki_shared_init", SCRIPTS / "wiki_shared_init.py"
)
shared = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shared)


class WikiSharedInitTest(unittest.TestCase):
    def test_creates_separate_repo_wiki_and_archive_without_overwriting(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            repo = root / "repo"
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            wiki, archive = shared.initialize(repo)
            self.assertTrue((wiki / "raw").is_dir())
            self.assertTrue((wiki / "wiki").is_dir())
            self.assertTrue(archive.is_dir())
            (wiki / "valuable").write_text("keep")
            with self.assertRaises(ValueError):
                shared.initialize(repo)
            self.assertEqual((wiki / "valuable").read_text(), "keep")

    def test_unignored_alternate_inside_git_is_rejected_before_creation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            repo = root / "repo"
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            alternate = repo / "data"
            alternate.mkdir()
            with self.assertRaises(ValueError):
                shared.initialize(repo, str(alternate))
            self.assertEqual(list(alternate.iterdir()), [])
            (repo / ".git/info/exclude").write_text("/data/\n")
            with self.assertRaises(ValueError):
                shared.initialize(repo, str(alternate))
            self.assertEqual(list(alternate.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
