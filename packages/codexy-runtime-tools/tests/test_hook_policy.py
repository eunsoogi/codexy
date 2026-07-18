import json
import time
import unittest
from pathlib import Path

from codexy_runtime_tools.hook_policy import MAX_INPUT_BYTES, evaluate
from codexy_runtime_tools.shell_policy import shell_forbidden


def encoded(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False).encode()


class HookPolicyTests(unittest.TestCase):
    def test_unrelated_and_other_repository_surfaces_are_zero_byte_noops(self) -> None:
        unrelated = {
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__filesystem__read_file",
            "tool_input": {"path": "/tmp/example"},
        }
        other_repo = {
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codex_apps__github_create_issue",
            "tool_input": {
                "repository_full_name": "example/elsewhere",
                "title": "fix: intentionally irrelevant",
            },
        }
        self.assertEqual(evaluate("PreToolUse", encoded(unrelated)), b"")
        self.assertEqual(evaluate("PreToolUse", encoded(other_repo)), b"")

    def test_owned_invalid_and_permission_requests_use_event_native_denials(self) -> None:
        invalid_issue = {
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codex_apps__github_create_issue",
            "tool_input": {
                "repository_full_name": "eunsoogi/codexy",
                "title": "fix: invalid title",
            },
        }
        pretool = json.loads(evaluate("PreToolUse", encoded(invalid_issue)))
        self.assertEqual(pretool["hookSpecificOutput"]["hookEventName"], "PreToolUse")
        self.assertEqual(pretool["hookSpecificOutput"]["permissionDecision"], "deny")

        shell = {
            "hook_event_name": "PermissionRequest",
            "tool_name": "Bash",
            "cwd": str(Path.cwd()),
            "tool_input": {"command": "sh -c 'git push --force origin main'"},
        }
        permission = json.loads(evaluate("PermissionRequest", encoded(shell)))
        self.assertEqual(
            permission["hookSpecificOutput"]["decision"]["behavior"], "deny"
        )
        rendered = json.dumps(permission)
        for forbidden in [
            "allow", "ask", "additionalContext", "systemMessage", "updatedInput"
        ]:
            self.assertNotIn(forbidden, rendered)

    def test_typed_json_limits_and_shell_structure_fail_closed_quickly(self) -> None:
        duplicate = b'{"hook_event_name":"PreToolUse","hook_event_name":"PreToolUse"}'
        self.assertTrue(evaluate("PreToolUse", duplicate))
        self.assertTrue(evaluate("PreToolUse", b"{" + b" " * MAX_INPUT_BYTES))

        safe = {
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "cwd": str(Path.cwd()),
            "tool_input": {"command": "printf '%s\\n' 'git push --force'"},
        }
        started = time.monotonic()
        self.assertEqual(evaluate("PreToolUse", encoded(safe)), b"")
        self.assertLess(time.monotonic() - started, 2.0)

    def test_shell_wrappers_and_benign_lookalikes_are_structural(self) -> None:
        denied = [
            "git push origin main --force-with-lease",
            "env CI=1 command git push -f origin main",
            "git push origin +main:main",
            "sudo gh pr merge 453 --admin --squash",
            "sh -c 'git push --force origin main'",
            'pwsh -Command "git push --force origin main"',
            "echo ok && git reset --hard HEAD~1",
            "rm -rf /",
            "command -- git push --force origin main",
            "sudo -u root git push --force origin main",
            "git -C /tmp push --force origin main",
            "git --no-pager reset --hard HEAD~1",
            "env -u GIT_CONFIG git push --force origin main",
            "gh --repo eunsoogi/codexy pr merge 453 --admin",
        ]
        allowed = [
            "printf '%s\\n' 'git push --force'",
            "rg -- '--admin' docs/force-push.md",
            "git push origin feature/force-push-docs",
            "rm -rf ./target/hook-fixture",
        ]
        for command in denied:
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "cwd": str(Path.cwd()),
                "tool_input": {"command": command},
            }
            self.assertTrue(evaluate("PreToolUse", encoded(payload)), command)
        for command in allowed:
            payload = {
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "cwd": str(Path.cwd()),
                "tool_input": {"command": command},
            }
            self.assertEqual(evaluate("PreToolUse", encoded(payload)), b"", command)

    def test_wrapper_and_global_option_shapes_are_structural(self) -> None:
        for command in [
            "env --unknown git push --force origin main",
            "sudo --unknown git push --force origin main",
            "git --unknown push --force origin main",
            "gh --unknown pr merge 453 --admin",
        ]:
            self.assertTrue(shell_forbidden(command), command)
        for command in [
            "command -- git push origin feature/force-push-docs",
            "sudo -u runner git push origin feature/force-push-docs",
            "git -C /tmp push origin feature/force-push-docs",
            "env -u GIT_CONFIG git push origin feature/force-push-docs",
            "gh --repo example/elsewhere pr merge 453 --squash",
        ]:
            self.assertFalse(shell_forbidden(command), command)

    def test_wrong_event_and_secret_fields_deny_without_echoing_input(self) -> None:
        secret = "CODEXY_SECRET_CANARY_453"
        payload = {
            "hook_event_name": "PermissionRequest",
            "tool_name": "mcp__codex_apps__github_create_issue",
            "tool_input": {
                "repository_full_name": "eunsoogi/codexy",
                "title": "Valid descriptive issue",
                "description": secret,
            },
        }
        output = evaluate("PreToolUse", encoded(payload))
        self.assertTrue(output)
        self.assertNotIn(secret.encode(), output)


if __name__ == "__main__":
    unittest.main()
