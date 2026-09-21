from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from github_native_hook_support import PLUGIN, native_command


class GithubTitleNestedShellTests(unittest.TestCase):
    def check_code(self, code: str, denied: bool, cwd: str | None = None) -> None:
        for event in ("PreToolUse", "PermissionRequest"):
            with self.subTest(event=event, code=code):
                result = subprocess.run(
                    native_command(
                        [str(PLUGIN / "hooks/codexy-title-check.sh"), event, "nested"]
                    ),
                    input=json.dumps(
                        {
                            "hook_event_name": event,
                            "tool_name": "functions.exec",
                            "tool_input": {"code": code},
                            "cwd": cwd,
                        }
                    ),
                    env={**os.environ, "PLUGIN_ROOT": str(PLUGIN)},
                    text=True,
                    capture_output=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(bool(result.stdout), denied, result.stdout)
                if denied:
                    output = json.loads(result.stdout)["hookSpecificOutput"]
                    decision = (
                        output["decision"]["behavior"]
                        if event == "PermissionRequest"
                        else output["permissionDecision"]
                    )
                    self.assertEqual(decision, "deny")

    def test_literal_commands_reuse_title_rules(self) -> None:
        for command, denied in (
            ("gh pr merge 42 --squash --subject 'fix(hooks): preserve title'", True),
            (
                "gh pr merge 42 --squash --subject 'fix(hooks): preserve title (#41)'",
                True,
            ),
            (
                "gh pr merge 42 --squash --subject 'fix(hooks): preserve title (#42)' --body 'free form'",
                False,
            ),
            ("gh issue create --title 'fix: invalid'", True),
            ("gh issue create --title 'Valid issue'", False),
            ("gh pr create --title 'plain title'", True),
            ("gh pr create --title 'fix(hooks): valid title'", False),
            ("gh pr view 42", False),
            ("gh release create v1", False),
            ("printf '%s' 'gh pr create --title invalid'", False),
        ):
            self.check_code(
                f"await tools.exec_command({json.dumps({'cmd': command})});", denied
            )

    def test_workdir_and_inherited_cwd_resolve_api_input(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            nested = root / "nested"
            nested.mkdir()
            (root / "payload.json").write_text(json.dumps({"title": "plain title"}))
            (nested / "payload.json").write_text(json.dumps({"title": "Valid issue"}))
            command = "gh api repos/o/r/issues --input payload.json"
            self.check_code(
                f"await tools.exec_command({json.dumps({'cmd': command})});",
                True,
                directory,
            )
            for workdir in (str(nested), "nested"):
                self.check_code(
                    f"await tools.exec_command({json.dumps({'cmd': command, 'workdir': workdir})});",
                    False,
                    directory,
                )

    def test_unrelated_dynamic_and_quoted_calls_are_not_admission_gates(self) -> None:
        for code in (
            "await tools.exec_command(options);",
            "await tools.exec_command({cmd: command});",
            'const example = "tools.exec_command({cmd: invalid})";',
            "// tools.exec_command({cmd: 'gh pr create --title invalid'})",
            "await tools.exec_command({cmd: 'python3 script.py'});",
        ):
            self.check_code(code, False)


if __name__ == "__main__":
    unittest.main()
