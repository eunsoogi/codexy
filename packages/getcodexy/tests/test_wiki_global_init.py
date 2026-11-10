"""Verify global initialization only against synthetic user data directories."""

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
    "wiki_global_init", SCRIPTS / "wiki_global_init.py"
)
global_init = importlib.util.module_from_spec(spec)
spec.loader.exec_module(global_init)


class WikiGlobalInitTest(unittest.TestCase):
    def test_configured_global_wiki_preserves_existing_data(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp).resolve()
            env = {"CODEXY_DATA_HOME": str(home / "data")}
            wiki = global_init.initialize(env, "Linux", home)
            self.assertTrue((wiki / "raw").is_dir())
            (wiki / "valuable").write_text("keep")
            with self.assertRaises(ValueError):
                global_init.initialize(env, "Linux", home)
            self.assertEqual((wiki / "valuable").read_text(), "keep")

    def test_global_inside_git_requires_existing_exclusion(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp).resolve()
            repo = home / "repo"
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            env = {"CODEXY_DATA_HOME": str(repo / "data")}
            with self.assertRaises(ValueError):
                global_init.initialize(env, "Linux", home)
            self.assertFalse((repo / "data").exists())
            (repo / ".git/info/exclude").write_text("/data/\n")
            self.assertTrue(global_init.initialize(env, "Linux", home).is_dir())


if __name__ == "__main__":
    unittest.main()
