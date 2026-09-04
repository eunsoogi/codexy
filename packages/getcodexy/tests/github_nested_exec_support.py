"""Installed-plugin parity cases for nested GitHub connector admission."""

import json
from pathlib import Path


def assert_nested_exec_cases(
    test_case: object,
    run_process: object,
    installed: Path,
    environment: dict[str, str],
    root: Path,
) -> None:
    nested_hook = installed / "hooks/codexy-repository-github-exec.sh"
    nested_cases = (
        (
            "PermissionRequest",
            'await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"Require typed nested admission"});',
            False,
        ),
        (
            "PermissionRequest",
            'await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"fix(hooks): bypass nested admission"});',
            True,
        ),
        (
            "PreToolUse",
            'await tools.mcp__codex_apps__github_create_pull_request({repository_full_name:"eunsoogi/codexy", title:"fix(hooks): admit nested GitHub calls", head_branch:"topic", base_branch:"main"});',
            False,
        ),
        (
            "PreToolUse",
            'await tools.mcp__codex_apps__github_create_pull_request({repository_full_name:"eunsoogi/codexy", title:`${title}`, head_branch:"topic", base_branch:"main"});',
            True,
        ),
        (
            "PreToolUse",
            "await tools.mcp__codex_apps__github_get_repo({repository_full_name:getRepository()});",
            False,
        ),
        (
            "PreToolUse",
            "await tools.some_other_tool({value:getValue()});",
            False,
        ),
        (
            "PermissionRequest",
            'await tools?.[fullName]({repository_full_name:"eunsoogi/codexy", title:"Create optional nested issue"});',
            True,
        ),
        (
            "PreToolUse",
            'await tools["mcp__codex_apps__" + "github_create_issue"]({repository_full_name:"eunsoogi/codexy", title:"Create concatenated nested issue"});',
            True,
        ),
        (
            "PermissionRequest",
            '(Reflect.get)(tools, "mcp__codex_apps__github_create_issue")({repository_full_name:"eunsoogi/codexy", title:"Create reflected nested issue"});',
            True,
        ),
        (
            "PreToolUse",
            'eval("await tools.mcp__codex_apps__github_create_issue({repository_full_name:\\"eunsoogi/codexy\\", title:\\"Create evaluated nested issue\\"});");',
            True,
        ),
        (
            "PermissionRequest",
            'eval("await tools.mcp__codex_apps__github_create_issue({repository_full_name:\\"eunsoogi/codexy\\", title:\\"Create evaluated nested issue from permission\\"});");',
            True,
        ),
        (
            "PermissionRequest",
            '(eval)("await tools.mcp__codex_apps__github_create_issue({repository_full_name:\\"eunsoogi/codexy\\", title:\\"Create parenthesized evaluated nested issue\\"});");',
            True,
        ),
        (
            "PreToolUse",
            'eval?.("await tools.mcp__codex_apps__github_create_issue({repository_full_name:\\"eunsoogi/codexy\\", title:\\"Create optional evaluated nested issue\\"});");',
            True,
        ),
        (
            "PermissionRequest",
            'const {mcp__codex_apps__github_create_issue: create} = tools; create({repository_full_name:"eunsoogi/codexy", title:"Create destructured nested issue"});',
            True,
        ),
        (
            "PreToolUse",
            'const {mcp__codex_apps__github_create_issue: create} = tools; create({repository_full_name:"eunsoogi/codexy", title:"Create destructured nested issue from pre-tool"});',
            True,
        ),
        (
            "PermissionRequest",
            '`${tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"Create templated nested issue"})}`',
            True,
        ),
        (
            "PreToolUse",
            "const pattern = /mcp__codex_apps__github_create_issue/;",
            False,
        ),
    )
    for event, code, denied in nested_cases:
        with test_case.subTest(event=event, code=code):
            nested_payload = {
                "hook_event_name": event,
                "tool_name": "functions.exec",
                "tool_input": {"code": code},
                "cwd": str(root),
            }
            output = run_process(
                [str(nested_hook), event],
                json.dumps(nested_payload),
                {**environment, "PLUGIN_ROOT": str(installed)},
            )
            marker = (
                '"decision":{"behavior":"deny"'
                if event == "PermissionRequest"
                else '"permissionDecision":"deny"'
            )
            test_case.assertEqual(marker in output, denied, output)
