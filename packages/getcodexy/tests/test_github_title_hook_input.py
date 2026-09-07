from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from github_native_hook_support import PLUGIN, native_command


class GithubTitleHookInputTests(unittest.TestCase):
    def _run_process(self, payload: str, environment: dict[str, str]) -> str:
        result = subprocess.run(
            native_command(
                [str(PLUGIN / "hooks/codexy-title-check.sh"), "PreToolUse", "shell"]
            ),
            input=payload,
            text=True,
            capture_output=True,
            env=environment,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_input_payloads_use_request_cwd_and_never_leak_content(self) -> None:
        environment = {**os.environ, "PLUGIN_ROOT": str(PLUGIN)}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for filename, content in {
                "valid.json": {"title": "Valid issue", "body": "secret body"},
                "invalid.json": {"title": "plain title", "body": "secret body"},
                "body.json": {"body": "secret body"},
            }.items():
                (root / filename).write_text(json.dumps(content), encoding="utf-8")
            for filename, denied in (
                ("valid.json", False),
                ("invalid.json", True),
                ("missing.json", True),
                ("body.json", True),
            ):
                with self.subTest(filename=filename, denied=denied):
                    payload = json.dumps(
                        {
                            "hook_event_name": "PreToolUse",
                            "tool_name": "Bash",
                            "tool_input": {
                                "command": f"gh api repos/o/r/issues --input {filename}"
                            },
                            "cwd": str(root),
                        }
                    )
                    output = self._run_process(payload, environment)
                    self.assertEqual(bool(output), denied, output)
                    self.assertNotIn("secret body", output)
            update = json.dumps(
                {
                    "hook_event_name": "PreToolUse",
                    "tool_name": "Bash",
                    "tool_input": {
                        "command": "gh api repos/o/r/issues/42 --input body.json"
                    },
                    "cwd": str(root),
                }
            )
            self.assertEqual(self._run_process(update, environment), "")


if __name__ == "__main__":
    unittest.main()
