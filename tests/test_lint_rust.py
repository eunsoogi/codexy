"""Unit tests for changed-file Rust diagnostic filtering."""

from __future__ import annotations

import importlib.util
import io
import json
import os
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "lint-rust.py"


def load_lint_rust():
    spec = importlib.util.spec_from_file_location("lint_rust", SCRIPT)
    if spec is None or spec.loader is None:
        raise AssertionError("Rust lint filter is not importable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RustLintTests(unittest.TestCase):
    def test_only_changed_primary_spans_fail_the_gate(self) -> None:
        lint_rust = load_lint_rust()
        changed = ROOT / "packages/codexy-runtime/src/changed.rs"
        unchanged = ROOT / "packages/codexy-runtime/src/unchanged.rs"
        messages = "\n".join(
            json.dumps(
                {
                    "reason": "compiler-message",
                    "message": {
                        "level": "warning",
                        "message": label,
                        "spans": [{"file_name": str(path), "is_primary": True}],
                    },
                }
            )
            for label, path in (
                ("changed warning", changed),
                ("old warning", unchanged),
            )
        )

        diagnostics = lint_rust.changed_diagnostics(
            messages,
            ROOT,
            ROOT / "packages/codexy-runtime",
            {changed.relative_to(ROOT).as_posix()},
        )

        self.assertEqual([item["message"] for item in diagnostics], ["changed warning"])

    def test_package_relative_primary_span_matches_a_changed_source(self) -> None:
        lint_rust = load_lint_rust()
        for level in ("warning", "error"):
            with self.subTest(level=level):
                message = json.dumps(
                    {
                        "reason": "compiler-message",
                        "message": {
                            "level": level,
                            "message": f"package-relative {level}",
                            "spans": [
                                {"file_name": "src/changed.rs", "is_primary": True}
                            ],
                        },
                    }
                )

                diagnostics = lint_rust.changed_diagnostics(
                    message,
                    ROOT,
                    ROOT / "packages/codexy-runtime",
                    {"packages/codexy-runtime/src/changed.rs"},
                )

                self.assertEqual(
                    [item["message"] for item in diagnostics],
                    [f"package-relative {level}"],
                )

    def test_main_rejects_a_package_relative_changed_source_diagnostic(self) -> None:
        lint_rust = load_lint_rust()
        output = json.dumps(
            {
                "reason": "compiler-message",
                "message": {
                    "level": "error",
                    "message": "changed package source",
                    "spans": [
                        {
                            "file_name": "tests/repository_eol_contract.rs",
                            "is_primary": True,
                        }
                    ],
                },
            }
        )
        completed = SimpleNamespace(returncode=0, stdout=output, stderr="")
        arguments = [
            "lint-rust.py",
            "--manifest-path",
            "packages/codexy-runtime/Cargo.toml",
            "packages/codexy-runtime/tests/repository_eol_contract.rs",
        ]

        with (
            mock.patch.object(lint_rust.subprocess, "run", return_value=completed),
            mock.patch.object(lint_rust, "changed_line_numbers", return_value=None),
            mock.patch("sys.argv", arguments),
        ):
            self.assertEqual(lint_rust.main(), 1)

    def test_changed_line_scope_ignores_old_lines_in_a_changed_source(self) -> None:
        lint_rust = load_lint_rust()
        path = "packages/codexy-runtime/src/changed.rs"
        messages = "\n".join(
            json.dumps(
                {
                    "reason": "compiler-message",
                    "message": {
                        "level": "warning",
                        "message": label,
                        "spans": [
                            {
                                "file_name": "src/changed.rs",
                                "is_primary": True,
                                "line_start": line,
                                "line_end": line,
                            }
                        ],
                    },
                }
            )
            for label, line in (("changed line", 10), ("old line", 2))
        )

        diagnostics = lint_rust.changed_diagnostics(
            messages,
            ROOT,
            ROOT / "packages/codexy-runtime",
            {path},
            {path: {10}},
        )

        self.assertEqual([item["message"] for item in diagnostics], ["changed line"])

    def test_changed_line_numbers_reads_merge_base_hunks(self) -> None:
        lint_rust = load_lint_rust()
        output = "\n".join(
            (
                "diff --git a/packages/codexy-runtime/src/changed.rs b/packages/codexy-runtime/src/changed.rs",
                "+++ b/packages/codexy-runtime/src/changed.rs",
                "@@ -3 +10,2 @@",
                "@@ -20,2 +30 @@",
            )
        )
        completed = SimpleNamespace(stdout=output)

        with (
            mock.patch.dict(os.environ, {"CODEXY_LINT_CHANGED_SINCE": "main"}),
            mock.patch.object(lint_rust.subprocess, "run", return_value=completed),
        ):
            lines = lint_rust.changed_line_numbers(
                ROOT, {"packages/codexy-runtime/src/changed.rs"}
            )

        self.assertEqual(
            lines, {"packages/codexy-runtime/src/changed.rs": {10, 11, 30}}
        )

    def test_diagnostics_include_primary_source_locations(self) -> None:
        lint_rust = load_lint_rust()
        stream = io.StringIO()
        diagnostic = {
            "message": "changed lint",
            "spans": [
                {
                    "file_name": "src/changed.rs",
                    "is_primary": True,
                    "line_start": 10,
                    "column_start": 4,
                }
            ],
        }

        with mock.patch("sys.stderr", stream):
            lint_rust.print_diagnostics([diagnostic])

        self.assertEqual(stream.getvalue(), "src/changed.rs:10:4: changed lint\n")


if __name__ == "__main__":
    unittest.main()
