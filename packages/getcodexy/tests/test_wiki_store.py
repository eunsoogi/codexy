"""Exercise serialized Wiki revisions in disposable local storage."""

import concurrent.futures
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPTS = (
    Path(__file__).resolve().parents[3] / "plugins/codexy/skills/"
    "self-improvement/scripts"
)
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("wiki_store", SCRIPTS / "wiki_store.py")
store = importlib.util.module_from_spec(spec)
spec.loader.exec_module(store)
brief_spec = importlib.util.spec_from_file_location(
    "brief_fixture", Path(__file__).with_name("test_continuation_brief.py")
)
brief_fixture = importlib.util.module_from_spec(brief_spec)
brief_spec.loader.exec_module(brief_fixture)


def candidate(identifier, title):
    return {
        "id": identifier,
        "sources": ["raw/source.md@revision"],
        "changes": {"wiki/article.md": f"# {title}\n", "_index.md": f"{title}\n"},
        "operation": f"Recorded {title}",
        "checkpoint": "evidence-1",
    }


class WikiStoreTest(unittest.TestCase):
    def fixture(self, tmp):
        root = Path(tmp).resolve() / "wiki"
        (root / "wiki").mkdir(parents=True)
        (root / "raw").mkdir()
        (root / "raw/source.md").write_text("original evidence")
        (root / "_index.md").write_text("old index")
        (root / "log.md").write_text("# log\n")
        (root / "config.md").write_text("config")
        return root

    def test_import_is_additive_and_commits_preserve_raw_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            original = (root / "raw/source.md").read_bytes()
            base = store.initialize(root)
            first = store.commit(root, base, candidate("a", "lesson"))
            revision, contents = store.read_current(root)
            self.assertEqual(revision, first)
            self.assertEqual(contents["raw/source.md"], original)
            self.assertEqual(contents["wiki/article.md"], b"# lesson\n")
            self.assertEqual((root / "_index.md").read_text(), "old index")
            self.assertEqual(store.commit(root, base, candidate("a", "lesson")), first)
            with self.assertRaises(ValueError):
                store.commit(root, first, candidate("a", "changed"))
            with self.assertRaises(ValueError):
                store.initialize(root)

    def test_competing_writers_detect_stale_revision_and_keep_both_candidates(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            base = store.initialize(root)
            candidates = [candidate("a", "first"), candidate("b", "second")]

            def run(item):
                try:
                    return store.commit(root, base, item)
                except ValueError:
                    return None

            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                results = list(executor.map(run, candidates))
            self.assertEqual(sum(item is not None for item in results), 1)
            loser = candidates[results.index(None)]
            current, _ = store.read_current(root)
            store.commit(root, current, loser)
            _, contents = store.read_current(root)
            self.assertIn(loser["operation"].encode(), contents["log.md"])

    def test_cli_import_commit_and_bounded_read(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            cli = SCRIPTS / "wiki_store_cli.py"

            def invoke(*args):
                return subprocess.run(
                    [sys.executable, str(cli), *map(str, args)],
                    check=True,
                    text=True,
                    stdout=subprocess.PIPE,
                ).stdout.strip()

            base = invoke("initialize", root)
            proposal = Path(tmp) / "candidate.json"
            proposal.write_text(json.dumps(candidate("a", "lesson")))
            revision = invoke(
                "commit", root, "--expected", base, "--candidate", proposal
            )
            result = json.loads(invoke("read", root, "--path", "wiki/article.md"))
            self.assertEqual(result["revision"], revision)
            self.assertEqual(result["content"], "# lesson\n")
            self.assertEqual(invoke("verify", root), revision)

    def test_brief_refresh_cannot_drop_a_correction_or_unresolved_condition(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            base = store.initialize(root)
            original = brief_fixture.sample()
            first = candidate("brief-a", "checkpoint")
            first["changes"]["wiki/continuation.json"] = json.dumps(original)
            current = store.commit(root, base, first)
            revised = brief_fixture.sample()
            revised["corrections"] = []
            second = candidate("brief-b", "refresh")
            second["changes"]["wiki/continuation.json"] = json.dumps(revised)
            with self.assertRaises(ValueError):
                store.commit(root, current, second)
            self.assertEqual(store.read_current(root)[0], current)
            revised["corrections"] = original["corrections"]
            revised["remaining_conditions"] = []
            second["changes"]["wiki/continuation.json"] = json.dumps(revised)
            with self.assertRaises(ValueError):
                store.commit(root, current, second)
            self.assertEqual(store.read_current(root)[0], current)

    def test_failed_publication_preserves_checkpoint_and_retry_succeeds(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            base = store.initialize(root)
            with patch.object(
                store.os, "replace", side_effect=OSError("injected failure")
            ):
                with self.assertRaises(OSError):
                    store.commit(root, base, candidate("a", "lesson"))
            self.assertEqual(store.read_current(root)[0], base)
            self.assertNotEqual(
                store.commit(root, base, candidate("a", "lesson")), base
            )

    def test_failed_additive_import_leaves_original_tree_untouched(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            before = {
                p.relative_to(root): p.read_bytes()
                for p in root.rglob("*")
                if p.is_file()
            }
            with patch.object(
                store, "rename_new", side_effect=OSError("injected failure")
            ):
                with self.assertRaises(OSError):
                    store.initialize(root)
            after = {
                p.relative_to(root): p.read_bytes()
                for p in root.rglob("*")
                if p.is_file()
            }
            self.assertEqual(after, before)
            self.assertFalse((root / ".codexy").exists())
            self.assertTrue(store.initialize(root))

    def test_exclusive_import_never_replaces_an_existing_directory(self):
        from wiki_exclusive_rename import rename_new

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            source, destination = root / "stage", root / "existing"
            source.mkdir()
            destination.mkdir()
            (destination / "valuable").write_text("keep")
            with self.assertRaises(OSError):
                rename_new(source, destination)
            self.assertEqual((destination / "valuable").read_text(), "keep")

    def test_source_change_during_import_stops_without_replacing_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            original_publish = store.publish

            def change_source(*args):
                original_publish(*args)
                (root / "raw/source.md").write_text("newer evidence")

            with patch.object(store, "publish", side_effect=change_source):
                with self.assertRaises(ValueError):
                    store.initialize(root)
            self.assertEqual((root / "raw/source.md").read_text(), "newer evidence")
            self.assertFalse((root / ".codexy").exists())

    def test_commit_reads_only_needed_evidence_and_bounded_lookup_detects_corruption(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            (root / "raw/other.md").write_text("independent fact")
            base = store.initialize(root)
            calls = []
            original = store.read_file

            def observe(directory, name, *args):
                calls.append(name)
                return original(directory, name, *args)

            with patch.object(store, "read_file", side_effect=observe):
                current = store.commit(root, base, candidate("a", "lesson"))
            self.assertNotIn("raw/source.md", calls)
            self.assertNotIn("raw/other.md", calls)
            self.assertEqual(
                store.read_one(root, "raw/other.md"), (current, b"independent fact")
            )
            with self.assertRaises(ValueError):
                store.read_one(root, "raw/other.md", 4)
            with self.assertRaises(ValueError):
                store.read_one(root, "raw/missing.md")

    def test_protected_paths_raw_overwrite_and_missing_evidence_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.fixture(tmp)
            base = store.initialize(root)
            for path in ("../AGENTS.md", "raw/source.md", "settings.json"):
                item = candidate("unsafe", "lesson")
                item["changes"][path] = "replacement"
                with self.subTest(path=path), self.assertRaises(ValueError):
                    store.commit(root, base, item)
            missing = candidate("missing", "lesson")
            missing["sources"] = []
            with self.assertRaises(ValueError):
                store.commit(root, base, missing)
            self.assertEqual(store.read_current(root)[0], base)


if __name__ == "__main__":
    unittest.main()
