from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).parents[3]
FIXTURES = Path(__file__).with_name("batch_change_fixtures")
SCRIPT = (
    ROOT
    / "plugins/codexy/skills/engineering/scripts/batch_change_input/batch_change_input.py"
)
sys.path.insert(0, str(SCRIPT.parent))

from manifest import preview_from_path  # noqa: E402
from preview import InputError, preview  # noqa: E402


class BatchChangeInputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "workspace"
        self.root.mkdir()

    def _manifest(self, fixture: str) -> Path:
        manifest = self.root / "batch.json"
        shutil.copy2(FIXTURES / fixture, manifest)
        return manifest

    def _run(
        self,
        manifest: Path,
        workspace: Path | None = None,
        script: Path = SCRIPT,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(script),
                "--workspace-root",
                str(workspace or self.root),
                "--input",
                str(manifest),
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_preview_records_state_commands_and_space_paths_without_side_effects(
        self,
    ) -> None:
        first = self.root / "input file.txt"
        second = self.root / "second.txt"
        first.write_text("first\n", encoding="utf-8")
        second.write_text("second\n", encoding="utf-8")
        before = {path: path.read_bytes() for path in (first, second)}
        manifest = self._manifest("valid.json")
        sentinel = self.root / "command-was-run"
        document = json.loads(manifest.read_text(encoding="utf-8"))
        document["items"][0]["transform"]["argv"] = [
            "touch",
            str(sentinel),
            "input file.txt",
        ]
        manifest.write_text(json.dumps(document), encoding="utf-8")
        result = self._run(manifest)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["schema"], "codexy.batch-change-input.v1")
        self.assertEqual(payload["status"], "preview")
        item = payload["items"][0]
        self.assertEqual(item["id"], "space-preserving-item")
        self.assertEqual(item["original"]["path"], "input file.txt")
        self.assertEqual(item["output"]["path"], "output file.txt")
        self.assertEqual(
            item["original"]["state"]["sha256"], hashlib.sha256(b"first\n").hexdigest()
        )
        self.assertEqual(item["commands"]["transform"]["argv"][2], "input file.txt")
        self.assertEqual(item["commands"]["validations"][0]["timeout_seconds"], 10)
        self.assertFalse(sentinel.exists() or (self.root / "output file.txt").exists())
        self.assertEqual({path: path.read_bytes() for path in before}, before)
        isolated_script = Path(self.temporary.name) / "isolated-script"
        shutil.copytree(
            SCRIPT.parent,
            isolated_script,
            ignore=shutil.ignore_patterns("__pycache__"),
        )
        script_before = sorted(
            path.relative_to(isolated_script) for path in isolated_script.rglob("*")
        )
        isolated_result = self._run(manifest, script=isolated_script / SCRIPT.name)
        self.assertEqual(
            isolated_result.returncode,
            0,
            isolated_result.stdout + isolated_result.stderr,
        )
        script_after = sorted(
            path.relative_to(isolated_script) for path in isolated_script.rglob("*")
        )
        self.assertEqual(script_before, script_after)

    def test_rejects_duplicate_outputs_and_interdependent_items(self) -> None:
        for fixture, expected in (
            ("duplicate_outputs.json", "duplicate output"),
            ("interdependent.json", "depends"),
        ):
            with self.subTest(fixture=fixture):
                for filename in ("first.txt", "second.txt"):
                    (self.root / filename).write_text(filename, encoding="utf-8")
                result = self._run(self._manifest(fixture))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected, json.loads(result.stdout)["error"])

    def test_rejects_escapes_invalid_argv_and_symlinked_files(self) -> None:
        (self.root / "first.txt").write_text("first", encoding="utf-8")
        escaped = self._run(self._manifest("escaped_original.json"))
        self.assertNotEqual(escaped.returncode, 0)
        self.assertIn("escapes", json.loads(escaped.stdout)["error"])
        invalid = self._run(self._manifest("invalid_argv.json"))
        self.assertNotEqual(invalid.returncode, 0)
        self.assertIn("argv", json.loads(invalid.stdout)["error"])
        outside = Path(self.temporary.name) / "outside.txt"
        outside.write_text("outside", encoding="utf-8")
        linked = self.root / "linked.txt"
        try:
            linked.symlink_to(outside)
        except (NotImplementedError, OSError) as error:
            self.skipTest(f"symlinks unavailable: {error}")
        document = json.loads((FIXTURES / "valid.json").read_text(encoding="utf-8"))
        document["items"] = [dict(document["items"][0], original="linked.txt")]
        manifest = self.root / "symlink.json"
        manifest.write_text(json.dumps(document), encoding="utf-8")
        linked_result = self._run(manifest)
        self.assertNotEqual(linked_result.returncode, 0)
        self.assertIn("symlink", json.loads(linked_result.stdout)["error"])

    def test_rejects_an_input_list_that_changes_during_preview(self) -> None:
        (self.root / "first.txt").write_text("first", encoding="utf-8")
        manifest = self._manifest("invalid_argv.json")
        contents = manifest.read_bytes()

        def mutate() -> bytes:
            manifest.write_bytes(contents + b" ")
            return contents

        with patch.object(Path, "read_bytes", side_effect=mutate):
            with self.assertRaisesRegex(InputError, "input list changed"):
                preview_from_path(self.root, str(manifest))

    def test_rejects_invalid_time_limits(self) -> None:
        (self.root / "input file.txt").write_text("first", encoding="utf-8")
        document = json.loads((FIXTURES / "valid.json").read_text(encoding="utf-8"))
        document["items"] = [document["items"][0]]
        document["items"][0]["transform"]["timeout_seconds"] = 0
        with self.assertRaisesRegex(InputError, "timeout_seconds"):
            preview(self.root, document)
        document["items"][0]["transform"]["timeout_seconds"] = 10**400
        with self.assertRaisesRegex(InputError, "timeout_seconds"):
            preview(self.root, document)
        document["items"][0]["transform"]["timeout_seconds"] = 30
        manifest = self._manifest("valid.json")
        manifest.write_text(
            json.dumps(document).replace(
                '"timeout_seconds": 30',
                '"timeout_seconds": ' + "1" * 5000,
                1,
            ),
            encoding="utf-8",
        )
        result = self._run(manifest)
        self.assertEqual(result.returncode, 2)
        self.assertNotIn("Traceback", result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "error")

    def test_rejects_alias_hard_link_and_nested_output_conflicts(self) -> None:
        first = self.root / "first.txt"
        second = self.root / "second.txt"
        first.write_text("first", encoding="utf-8")
        second.write_text("second", encoding="utf-8")
        document = json.loads((FIXTURES / "valid.json").read_text(encoding="utf-8"))
        base_item = document["items"][0]

        def item(item_id: str, original: str, output: str) -> dict[str, object]:
            return dict(base_item, id=item_id, original=original, output=output)

        hard_link = self.root / "alias.txt"
        try:
            os.link(first, hard_link)
        except OSError as error:
            self.skipTest(f"hard links unavailable: {error}")
        document["items"] = [item("same-original", "first.txt", "alias.txt")]
        with self.assertRaisesRegex(InputError, "replaces its original"):
            preview(self.root, document)
        output_one = self.root / "output-one.txt"
        output_two = self.root / "output-two.txt"
        output_one.write_text("output", encoding="utf-8")
        os.link(output_one, output_two)
        document["items"] = [
            item("first-output", "first.txt", "output-one.txt"),
            item("second-output", "second.txt", "output-two.txt"),
        ]
        with self.assertRaisesRegex(InputError, "duplicate output"):
            preview(self.root, document)
        original_alias = self.root / "first-alias.txt"
        os.link(first, original_alias)
        document["items"] = [
            item("first-original", "first.txt", "new-output-one.txt"),
            item("aliased-original", "first-alias.txt", "new-output-two.txt"),
        ]
        with self.assertRaisesRegex(InputError, "duplicate original"):
            preview(self.root, document)
        document["items"] = [
            item("first-item", "first.txt", "results/One.txt"),
            item("second-item", "second.txt", "results/one.txt"),
        ]
        with self.assertRaisesRegex(InputError, "duplicate output"):
            preview(self.root, document)
        document["items"][1]["output"] = "results/One.txt/child.txt"
        with self.assertRaisesRegex(InputError, "overlap"):
            preview(self.root, document)
        document["items"] = [item("nested-item", "first.txt", "first.txt/child.txt")]
        with self.assertRaisesRegex(InputError, "non-directory"):
            preview(self.root, document)

    def test_rejects_a_symlinked_workspace_root(self) -> None:
        target = self.root / "target"
        target.mkdir()
        link = Path(self.temporary.name) / "linked-workspace"
        try:
            link.symlink_to(target, target_is_directory=True)
        except (NotImplementedError, OSError) as error:
            self.skipTest(f"symlinks unavailable: {error}")
        manifest = self._manifest("valid.json")
        result = self._run(manifest, link)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("real directory", json.loads(result.stdout)["error"])


if __name__ == "__main__":
    unittest.main()
