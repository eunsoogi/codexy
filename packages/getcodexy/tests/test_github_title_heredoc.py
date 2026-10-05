"""Exercise quoted heredoc data and executable shell input through the title hook."""

from __future__ import annotations

import json
import os
import subprocess
import unittest

from github_native_hook_support import PLUGIN, native_command


class GithubTitleHeredocTests(unittest.TestCase):
    def _run_hook(self, event: str, kind: str, tool: str, tool_input: dict) -> str:
        result = subprocess.run(
            native_command([str(PLUGIN / "hooks/codexy-title-check.sh"), event, kind]),
            input=json.dumps(
                {
                    "hook_event_name": event,
                    "tool_name": tool,
                    "tool_input": tool_input,
                }
            ),
            env={**os.environ, "PLUGIN_ROOT": str(PLUGIN)},
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_direct_heredoc_body_is_data_and_shell_scripts_stay_checked(self) -> None:
        body = "reviewer's body update\ngh pr create --title 'plain title'\n"
        cases = (
            (f"gh pr edit 1249 --body-file - <<'BODY'\n{body}BODY\n", False),
            (f"gh pr edit 1249 -F - <<'BODY'\n{body}BODY\n", False),
            (f"gh pr edit 1249 --body-file=- <<'BODY'\n{body}BODY\n", False),
            ("gh pr edit 1249 --body-file /tmp/body.md", False),
            (
                f"gh pr edit 1249 --title 'plain title' --body-file - <<'BODY'\n{body}BODY\n",
                True,
            ),
            (
                f"gh pr edit 1249 --title='plain title' --body-file - <<'BODY'\n{body}BODY\n",
                True,
            ),
            (
                f"gh pr edit 1249 -t 'plain title' --body-file - <<'BODY'\n{body}BODY\n",
                True,
            ),
            (
                f"gh pr edit 1249 --title 'fix(hooks): valid title' --body-file - <<'BODY'\n{body}BODY\n",
                False,
            ),
            (
                "sh -s <<'BODY'\ngh pr edit 1249 --title 'plain title'\nBODY\n",
                True,
            ),
            (
                "sh -c 'sh -s' <<'BODY'\ngh pr edit 1249 --title 'plain title'\nBODY\n",
                True,
            ),
            (
                "cat <<'BODY' | sh\ngh pr edit 1249 --title 'plain title'\nBODY\n",
                True,
            ),
        )
        for command, denied in cases:
            for event in ("PreToolUse", "PermissionRequest"):
                with self.subTest(event=event, command=command):
                    output = self._run_hook(
                        event, "shell", "Bash", {"command": command}
                    )
                    self.assertEqual(bool(output), denied, output)

    def test_nested_exec_heredoc_body_is_data(self) -> None:
        cases = (
            (
                "gh pr edit 1249 --body-file - <<'BODY'\n"
                "reviewer's body update\n"
                "gh pr create --title 'plain title'\n"
                "BODY\n",
                False,
            ),
            (
                "gh pr edit 1249 --body-file - <<BODY\n"
                "$(gh pr create --title 'plain title')\n"
                "BODY\n",
                True,
            ),
            (
                "cat <<'BODY' | sh\ngh pr edit 1249 --title 'plain title'\nBODY\n",
                True,
            ),
        )
        for command, denied in cases:
            code = f"await tools.exec_command({json.dumps({'cmd': command})});"
            for event in ("PreToolUse", "PermissionRequest"):
                with self.subTest(event=event, command=command):
                    output = self._run_hook(
                        event, "nested", "functions.exec", {"code": code}
                    )
                    self.assertEqual(bool(output), denied, output)


if __name__ == "__main__":
    unittest.main()
