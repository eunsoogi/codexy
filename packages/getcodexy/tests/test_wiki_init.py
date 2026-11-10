"""Exercise local Wiki initialization without touching personal repositories."""

import importlib.util
import concurrent.futures
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
spec = importlib.util.spec_from_file_location("wiki_init", SCRIPTS / "wiki_init.py")
wiki_init = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wiki_init)
import wiki_store


def git(root, *args, check=True):
    return subprocess.run(
        ["git", "-C", str(root), *args],
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


class WikiInitTest(unittest.TestCase):
    def test_linked_worktrees_are_separate_and_locally_excluded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            repo, first, second = (root / x for x in ("repo", "first", "second"))
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
            git(repo, "worktree", "add", "-qb", "first", str(first))
            git(repo, "worktree", "add", "-qb", "second", str(second))
            exclude = repo / ".git/info/exclude"
            original = exclude.read_bytes()
            self.assertEqual(wiki_init.init(first), first / ".wiki")
            self.assertEqual(wiki_init.init(second), second / ".wiki")
            self.assertTrue(exclude.read_bytes().startswith(original))
            self.assertEqual(exclude.read_bytes().splitlines().count(b"/.wiki/"), 1)
            for task in (first, second):
                self.assertEqual(
                    git(
                        task, "check-ignore", "-q", ".wiki/_index.md", check=False
                    ).returncode,
                    0,
                )
                self.assertEqual(git(task, "status", "--porcelain").stdout, b"")
                self.assertTrue((task / ".wiki/log.md").is_file())

            def write_task(task):
                local = task / ".wiki"
                revision = wiki_store.initialize(local)
                item = {
                    "id": task.name,
                    "sources": ["task source@revision"],
                    "changes": {"_index.md": task.name, "wiki/task.md": task.name},
                    "operation": "record local finding",
                    "checkpoint": task.name,
                }
                wiki_store.commit(local, revision, item)
                return wiki_store.read_one(local, "wiki/task.md")[1]

            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                self.assertEqual(
                    list(executor.map(write_task, (first, second))),
                    [b"first", b"second"],
                )

    def test_existing_content_and_tracked_wiki_are_preserved(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp).resolve() / "repo"
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            local = repo / ".wiki"
            local.mkdir()
            (local / "valuable").write_text("keep")
            exclude = repo / ".git/info/exclude"
            original = exclude.read_bytes()
            with self.assertRaises(ValueError):
                wiki_init.init(repo)
            self.assertEqual((local / "valuable").read_text(), "keep")
            self.assertEqual(exclude.read_bytes(), original)
            git(repo, "add", ".wiki/valuable")
            (local / "valuable").unlink()
            local.rmdir()
            with self.assertRaises(ValueError):
                wiki_init.init(repo)
            self.assertFalse(local.exists())
            self.assertEqual(exclude.read_bytes(), original)

    def test_linked_destination_is_not_followed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            repo, outside = root / "repo", root / "outside"
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            outside.mkdir()
            (outside / "valuable").write_text("keep")
            (repo / ".wiki").symlink_to(outside, target_is_directory=True)
            exclude = repo / ".git/info/exclude"
            original = exclude.read_bytes()
            with self.assertRaises(ValueError):
                wiki_init.init(repo)
            self.assertEqual((outside / "valuable").read_text(), "keep")
            self.assertEqual(exclude.read_bytes(), original)

    def test_explicit_non_git_task_is_separate_and_git_mode_cannot_be_bypassed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            task, repo = root / "task", root / "repo"
            task.mkdir()
            self.assertEqual(wiki_init.init_standalone(task), task / ".wiki")
            self.assertTrue((task / ".wiki/_index.md").is_file())
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            with self.assertRaises(ValueError):
                wiki_init.init_standalone(repo)
            self.assertFalse((repo / ".wiki").exists())


if __name__ == "__main__":
    unittest.main()
