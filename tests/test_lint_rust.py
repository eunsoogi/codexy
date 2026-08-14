"""Unit tests for changed-file Rust diagnostic filtering."""

from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


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
            messages, ROOT, {changed.relative_to(ROOT).as_posix()}
        )

        self.assertEqual([item["message"] for item in diagnostics], ["changed warning"])


if __name__ == "__main__":
    unittest.main()
