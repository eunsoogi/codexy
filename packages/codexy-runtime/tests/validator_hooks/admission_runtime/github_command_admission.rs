use serde_json::json;

use super::{TestResult, assert_event_case, plugin_root, repository};

const WORKFLOW_DISPATCH: &str =
    "gh workflow run plugin-version-bump.yml --repo eunsoogi/codexy -f version=1.6.4 -f issue=871";
const WORKFLOW_RERUN: &str = "gh run rerun 33827377016 --repo eunsoogi/codexy";
const READ_ONLY_COMPOSITES: &[&str] = &[
    "for field in headRefOid baseRefOid; do gh pr view 875 --repo eunsoogi/codexy --json \"$field\"; done",
    "for run in 33827377016; do gh run view \"$run\" --repo eunsoogi/codexy; done",
    "for pr in 879 883 884; do gh pr view \"$pr\" --repo eunsoogi/codexy --json headRefOid,statusCheckRollup | jq ...; done",
];
const CACHE_CLEANUP_AND_STAGE: &str =
    "find . -type d -name __pycache__ -prune -exec rm -rf {} + && git add -- plugins/codexy-github/hooks/codexy_policy/repository_github_command.py packages/codexy-runtime/tests/validator_hooks/admission_runtime/repository_policy_runtime.rs";

#[test]
fn issue_876_workflow_dispatch_is_admitted_for_both_events() -> TestResult {
    assert_admitted(WORKFLOW_DISPATCH)
}

#[test]
fn issue_876_workflow_rerun_is_admitted_for_both_events() -> TestResult {
    assert_admitted(WORKFLOW_RERUN)
}

#[test]
fn issue_876_first_read_only_composite_is_admitted_for_both_events() -> TestResult {
    assert_admitted(READ_ONLY_COMPOSITES[0])
}

#[test]
fn issue_876_second_read_only_composite_is_admitted_for_both_events() -> TestResult {
    assert_admitted(READ_ONLY_COMPOSITES[1])
}

#[test]
fn issue_876_third_read_only_composite_is_admitted_for_both_events() -> TestResult {
    assert_admitted(READ_ONLY_COMPOSITES[2])
}

#[test]
fn issue_876_cache_cleanup_and_named_stage_are_admitted_for_both_events() -> TestResult {
    assert_admitted(CACHE_CLEANUP_AND_STAGE)
}

#[test]
fn issue_876_literal_option_operand_after_separator_is_admitted() -> TestResult {
    assert_admitted("git add -- -A")
}

fn assert_admitted(command: &str) -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(&root, event, &owned, command, false, &[])?;
    }
    Ok(())
}

#[test]
fn issue_876_dangerous_boundaries_remain_denied_for_both_events() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let commands = [
        "gh workflow run deploy.yml --repo eunsoogi/codexy",
        "gh workflow run plugin-version-bump.yml --repo eunsoogi/codexy -f version=1.6.4 -f issue=871 --ref release",
        "gh run rerun 33827377016 --repo eunsoogi/codexy --failed",
        "gh run rerun 33827377016 --repo openai/codex",
        "git push --delete origin topic",
        "git push origin :topic",
        "git push --prune origin topic",
        "git push --all origin",
        "git push --tags origin",
        "git push --force-if-includes origin topic",
        "git push --force origin topic",
        "rm -rf plugins/codexy",
        "find . -type d -name .cache -delete",
        "git add -A",
        "git add .",
        "git add -- :/",
        "git add -- ':(top)README.md'",
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for command in commands {
            assert_event_case(&root, event, &owned, command, true, &[])?;
        }
    }
    Ok(())
}

#[test]
fn issue_876_denials_explain_rule_operation_and_remediation() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for (launcher, command, details) in [
        (
            "codexy-repository-github-command",
            "gh workflow run deploy.yml --repo eunsoogi/codexy",
            "rule=workflow-dispatch; operation=governed GitHub workflow dispatch; remediation=",
        ),
        (
            "codexy-repository-github-command",
            "gh pr view 875 --repo eunsoogi/codexy && gh workflow run deploy.yml --repo eunsoogi/codexy",
            "rule=workflow-dispatch; operation=governed GitHub workflow dispatch; remediation=",
        ),
        (
            "codexy-destructive-command",
            "git push --delete origin topic",
            "rule=git-remote-update; operation=Git remote update; remediation=",
        ),
        (
            "codexy-destructive-command",
            "find . -type d -name __pycache__ -prune -exec rm -rf {} + && git add -A",
            "rule=staging-scope; operation=local Git staging; remediation=",
        ),
        (
            "codexy-destructive-command",
            "rm -rf plugins/codexy",
            "rule=bounded-local-deletion; operation=recursive local deletion; remediation=",
        ),
    ] {
        for event in ["PermissionRequest", "PreToolUse"] {
            let input = json!({
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": {"command": command},
                "cwd": owned,
            });
            let output = super::super::concern_launchers::run_launcher(
                &root, launcher, event, &input, &[],
            )?;
            let denial: serde_json::Value = serde_json::from_slice(&output)?;
            let reason = if event == "PermissionRequest" {
                &denial["hookSpecificOutput"]["decision"]["message"]
            } else {
                &denial["hookSpecificOutput"]["permissionDecisionReason"]
            };
            let reason = reason.as_str().unwrap_or_default();
            assert!(reason.contains(details), "{reason}");
            assert!(reason.contains("remediation="), "{reason}");
        }
    }
    Ok(())
}
