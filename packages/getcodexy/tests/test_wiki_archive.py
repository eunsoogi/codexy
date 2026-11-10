"""Verify archives survive local retirement and reject unsafe inputs."""

import concurrent.futures
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = (
    Path(__file__).resolve().parents[3] / "plugins/codexy/skills/"
    "self-improvement/scripts/wiki_archive.py"
)
spec = importlib.util.spec_from_file_location("wiki_archive", SCRIPT)
wiki_archive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wiki_archive)


class WikiArchiveTest(unittest.TestCase):
    def test_evidence_and_shared_knowledge_survive_disposable_git_worktree_removal(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            repo, task = root / "repo", root / "task"
            subprocess.run(["git", "init", "-q", str(repo)], check=True)
            subprocess.run(
                [
                    "git",
                    "-C",
                    str(repo),
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.invalid",
                    "commit",
                    "--allow-empty",
                    "-qm",
                    "initial",
                ],
                check=True,
            )
            subprocess.run(
                ["git", "-C", str(repo), "worktree", "add", "-qb", "task", str(task)],
                check=True,
            )
            local = task / ".wiki"
            local.mkdir()
            (local / "raw").mkdir()
            (local / "wiki").mkdir()
            (local / "wiki/brief.md").write_text("continuation evidence")
            for name in ("_index.md", "log.md", "config.md"):
                (local / name).write_text("fixture\n")
            store_cli = SCRIPT.parent / "wiki_store_cli.py"
            subprocess.run(
                [sys.executable, str(store_cli), "initialize", str(local)], check=True
            )
            current = json.loads((local / ".codexy/CURRENT").read_text())
            prefix = f".codexy/revisions/{current['directory']}"
            state = json.loads((local / prefix / "state.json").read_text())
            selection = [".codexy/CURRENT", f"{prefix}/state.json"] + [
                f"{prefix}/{name}" for name in state["files"]
            ]
            common = repo / ".git"
            archives = common / "codexy/task-archives"
            archives.mkdir(parents=True)
            shared = common / "codexy/wiki"
            shared.mkdir()
            (shared / "shared.md").write_text("validated repository knowledge")
            with (common / "info/exclude").open("a") as exclude:
                exclude.write("\n/.wiki/\n")
            subprocess.run(
                ["git", "-C", str(task), "check-ignore", "-q", ".wiki/wiki/brief.md"],
                check=True,
            )
            pointer = wiki_archive.archive(
                local, archives, selection, "task branch revision"
            )
            manifest = json.loads(pointer.read_text())
            subprocess.run(
                ["git", "-C", str(repo), "worktree", "remove", str(task)], check=True
            )
            self.assertFalse(task.exists())
            wiki_archive.verify_archive(pointer)
            archived_root = archives / manifest["data_dir"] / "files"
            subprocess.run(
                [sys.executable, str(store_cli), "verify", str(archived_root)],
                check=True,
            )
            self.assertEqual(
                (archived_root / prefix / "wiki/brief.md").read_text(),
                "continuation evidence",
            )
            self.assertEqual(
                (shared / "shared.md").read_text(),
                "validated repository knowledge",
            )
            damaged = archived_root / prefix / "wiki/brief.md"
            damaged.chmod(0o600)
            damaged.write_text("corrupted")
            with self.assertRaises(ValueError):
                wiki_archive.verify_archive(pointer)

    def test_selected_material_survives_local_retirement_and_retry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            source, target = root / "worktree" / ".wiki", root / "shared" / "archives"
            source.mkdir(parents=True)
            target.mkdir(parents=True)
            (source / "brief.md").write_text(
                "unfinished condition and source reference"
            )
            (source / "private.md").write_text("not selected")
            first = wiki_archive.archive(
                source, target, ["brief.md"], "task branch revision"
            )
            second = wiki_archive.archive(
                source, target, ["brief.md"], "task branch revision"
            )
            self.assertEqual(first, second)
            manifest = json.loads(first.read_text())
            self.assertEqual(list(manifest["files"]), ["brief.md"])
            # This fixture owns the disposable local files, not a real worktree.
            (source / "brief.md").unlink()
            (source / "private.md").unlink()
            source.rmdir()
            preserved = target / manifest["data_dir"] / "files" / "brief.md"
            self.assertEqual(
                preserved.read_text(), "unfinished condition and source reference"
            )

    def test_parallel_identical_writers_publish_one_verified_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            source, target = root / "local", root / "archive"
            source.mkdir()
            target.mkdir()
            (source / "evidence").write_text("evidence")
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                results = list(
                    pool.map(
                        lambda _: wiki_archive.archive(
                            source, target, ["evidence"], "same task revision"
                        ),
                        range(2),
                    )
                )
            self.assertEqual(results[0], results[1])
            self.assertEqual(len(list(target.glob("*.json"))), 1)

    def test_unsafe_paths_and_conflicting_manifest_do_not_overwrite(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            source, target = root / "local", root / "archive"
            source.mkdir()
            target.mkdir()
            (source / "evidence").write_text("evidence")
            (source / "link").symlink_to(source / "evidence")
            for bad in ("../outside", "link"):
                with self.subTest(path=bad), self.assertRaises((ValueError, OSError)):
                    wiki_archive.archive(source, target, [bad], "task revision")
            pointer = wiki_archive.archive(
                source, target, ["evidence"], "task revision"
            )
            pointer.unlink()
            pointer.write_text("{}")
            with self.assertRaises(ValueError):
                wiki_archive.archive(source, target, ["evidence"], "task revision")
            self.assertEqual(pointer.read_text(), "{}")


if __name__ == "__main__":
    unittest.main()
