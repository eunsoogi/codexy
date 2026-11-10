"""Exercise Wiki identity resolution against disposable Git repositories."""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = (
    Path(__file__).resolve().parents[3] / "plugins/codexy/skills/"
    "self-improvement/scripts/wiki_paths.py"
)
sys.path.insert(0, str(SCRIPT.parent))
spec = importlib.util.spec_from_file_location("wiki_paths", SCRIPT)
wiki_paths = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wiki_paths)


def git(directory: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(directory), *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout.strip()


class WikiPathsTest(unittest.TestCase):
    def test_platform_defaults_and_explicit_absolute_override(self):
        home = Path("/synthetic/home")
        self.assertEqual(
            wiki_paths.global_wiki({}, "Darwin", home),
            home / "Library/Application Support/Codexy/wiki",
        )
        self.assertEqual(
            wiki_paths.global_wiki({}, "Linux", home), home / ".local/share/codexy/wiki"
        )
        self.assertEqual(
            wiki_paths.global_wiki({"XDG_DATA_HOME": "/data"}, "Linux", home),
            Path("/data/codexy/wiki"),
        )
        self.assertEqual(
            wiki_paths.global_wiki({"LOCALAPPDATA": "/app"}, "Windows", home),
            Path("/app/Codexy/wiki"),
        )
        self.assertEqual(
            wiki_paths.global_wiki({"CODEXY_DATA_HOME": "/custom"}, "Linux", home),
            Path("/custom/wiki"),
        )
        with self.assertRaises(ValueError):
            wiki_paths.global_wiki({"CODEXY_DATA_HOME": "relative"}, "Linux", home)
        with self.assertRaises(ValueError):
            wiki_paths.global_wiki({}, "Windows", home)
        with self.assertRaises(ValueError):
            wiki_paths.global_wiki(
                {"CODEXY_DATA_HOME": str(SCRIPT.parents[3])}, "Linux", home
            )

    def test_linked_worktrees_share_only_repository_scope_and_clone_is_separate(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            repo, linked, clone = (root / name for name in ("repo", "linked", "clone"))
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            git(
                repo,
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-qm",
                "initial",
            )
            git(repo, "worktree", "add", "-qb", "task", str(linked))
            subprocess.run(["git", "clone", "-q", str(repo), str(clone)], check=True)
            git(repo, "remote", "add", "origin", "https://example.invalid/same.git")
            git(
                clone, "remote", "set-url", "origin", "https://example.invalid/same.git"
            )
            (linked / "nested").mkdir()
            one = wiki_paths.git_scopes(repo)
            two = wiki_paths.git_scopes(linked / "nested")
            three = wiki_paths.git_scopes(clone)
            self.assertEqual(one["repository"], two["repository"])
            self.assertEqual(one["archive"], two["archive"])
            self.assertNotEqual(one["repository"], three["repository"])
            alternate = str(root / "explicit-repository-data")
            self.assertEqual(
                wiki_paths.git_scopes(repo, alternate)["repository"],
                wiki_paths.git_scopes(linked, alternate)["repository"],
            )
            self.assertNotEqual(
                wiki_paths.git_scopes(repo, alternate)["repository"],
                wiki_paths.git_scopes(clone, alternate)["repository"],
            )
            self.assertEqual(two["worktree"], linked / ".wiki")
            self.assertNotEqual(one["worktree"], two["worktree"])
            self.assertEqual(one["exclude"], two["exclude"])
            self.assertTrue((linked / ".git").is_file())
            self.assertFalse(Path(one["repository"]).exists())
            self.assertFalse(Path(two["worktree"]).exists())

    def test_non_git_does_not_infer_repository_or_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(ValueError):
                wiki_paths.git_scopes(Path(tmp))
            explicit = wiki_paths.standalone_scopes(Path(tmp).resolve())
            self.assertEqual(explicit["worktree"], Path(tmp).resolve() / ".wiki")
            self.assertNotIn("repository", explicit)
            with self.assertRaises(ValueError):
                wiki_paths.standalone_scopes(Path("relative"))
            self.assertEqual(os.listdir(tmp), [])

    def test_global_scope_cannot_point_inside_another_plugin_installation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            plugin = root / "other-plugin"
            (plugin / ".codex-plugin").mkdir(parents=True)
            (plugin / ".codex-plugin/plugin.json").write_text("{}")
            with self.assertRaises(ValueError):
                wiki_paths.global_wiki({"CODEXY_DATA_HOME": str(plugin)}, "Linux", root)


if __name__ == "__main__":
    unittest.main()
