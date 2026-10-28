"""Regressions for Bash null words and command identity after brace expansion."""

from __future__ import annotations

import importlib
import json
import os
import subprocess
import sys
import unittest
from collections.abc import Callable
from pathlib import Path
from typing import cast

_support = importlib.import_module("github_native_hook_support")
PLUGIN = cast(Path, getattr(_support, "PLUGIN"))
native_command = cast(
    Callable[[list[str]], list[str]], getattr(_support, "native_command")
)
sys.path.insert(0, str(PLUGIN / "hooks"))
_expansion = importlib.import_module("codexy_policy.shell_brace_expansion")
restore_word = cast(Callable[[str], str], getattr(_expansion, "restore"))
_segments = importlib.import_module("codexy_policy.shell_segments")
tokenize = cast(Callable[[str], list[str] | None], getattr(_segments, "tokenize"))
segments = cast(
    Callable[[str], tuple[tuple[str, ...], ...] | None],
    getattr(_segments, "segments"),
)


class ShellBraceExpansionNullWordTests(unittest.TestCase):
    def _classify(self, command: str) -> str:
        hook = PLUGIN / "hooks/codexy-destructive-command.sh"
        payload = json.dumps(
            {
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": {"command": command},
                "cwd": str(PLUGIN.parents[1]),
            }
        )
        result = subprocess.run(
            native_command([str(hook), "PreToolUse"]),
            input=payload,
            text=True,
            capture_output=True,
            env={**os.environ, "PLUGIN_ROOT": str(PLUGIN)},
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_unquoted_null_products_cannot_hide_destructive_commands(self) -> None:
        denied = (
            "git diff --stat && {,} rm -rf /tmp/codexy-review-unused",
            "git diff --stat && {,} git reset --hard HEAD",
        )
        # The hook classifies these inert strings; it never executes their commands.
        for command in denied:
            with self.subTest(command=command):
                self.assertIn('"permissionDecision":"deny"', self._classify(command))

    def test_null_normalization_retains_explicit_empty_word_provenance(self) -> None:
        cases = (
            ("printf x {,} sentinel", ["printf", "x", "sentinel"]),
            ('printf x ""{,} sentinel', ["printf", "x", "", "", "sentinel"]),
            ('printf x {,"",word}', ["printf", "x", "", "word"]),
            (
                '""{,} rm -rf /tmp/codexy-review-unused',
                ["", "", "rm", "-rf", "/tmp/codexy-review-unused"],
            ),
        )
        for command, expected in cases:
            with self.subTest(command=command):
                tokens = tokenize(command)
                if tokens is None:
                    self.fail(f"bounded brace form was treated as opaque: {command}")
                self.assertEqual([restore_word(token) for token in tokens], expected)

        # An explicit null command word stays in command position; it cannot expose the following word.
        parsed = segments('""{,} rm -rf /tmp/codexy-review-unused')
        self.assertIsNotNone(parsed)
        self.assertEqual(parsed[0][:3], ("", "", "rm"))

        # Nonempty quotes inside active brace choices remain outside this bounded grammar.
        self.assertIsNone(tokenize('printf x {"quoted,arm",word}'))
