"""Unit tests for changed-file Rust diagnostic filtering."""

from __future__ import annotations

import importlib.util
import json
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
            mock.patch("sys.argv", arguments),
        ):
            self.assertEqual(lint_rust.main(), 1)


if __name__ == "__main__":
    unittest.main()
