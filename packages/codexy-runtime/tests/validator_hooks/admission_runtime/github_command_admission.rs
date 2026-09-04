use serde_json::json;

use super::{TestResult, assert_event_case, plugin_root, repository};

const WORKFLOW_DISPATCH: &str =
    "gh workflow run plugin-version-bump.yml --repo eunsoogi/codexy -f version=1.6.4 -f issue=871";
const WORKFLOW_RERUN: &str = "gh run rerun 33827377016 --repo eunsoogi/codexy";
const READ_ONLY_COMPOSITES: &[&str] = &[
    "for field in headRefOid baseRefOid; do gh pr view 875 --repo eunsoogi/codexy --json \"$field\"; done",
    "for run in 33827377016; do gh run view \"$run\" --repo eunsoogi/codexy; done",
];
const CACHE_CLEANUP_AND_STAGE: &str =
    "find . -type d -name __pycache__ -prune -exec rm -rf {} + && git add -- plugins/codexy-github/hooks/codexy_policy/repository_github_command.py packages/codexy-runtime/tests/validator_hooks/admission_runtime/repository_policy_runtime.rs";

#[test]
fn issue_876_observed_safe_operations_are_admitted_for_both_events() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let commands = [
        WORKFLOW_DISPATCH,
        WORKFLOW_RERUN,
        CACHE_CLEANUP_AND_STAGE,
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for command in commands {
            assert_event_case(&root, event, &owned, command, false, &[])?;
        }
        for command in READ_ONLY_COMPOSITES {
            assert_event_case(&root, event, &owned, command, false, &[])?;
        }
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
            "codexy-destructive-command",
            "git push --delete origin topic",
            "rule=git-remote-update; operation=Git remote update; remediation=",
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
