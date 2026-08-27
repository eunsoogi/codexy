use std::io::Write as _;
use std::path::Path;
use std::process::Stdio;

use crate::support::FixtureCommand as Command;

use super::super::{copy_github as copy, read, text, validate};

const ISSUE_MATCHER: &str = "^mcp__codex_apps__github_(?:add_comment_to_issue|add_issue_assignees|add_issue_labels|add_reaction_to_issue_comment|add_reaction_to_pr|add_reaction_to_pr_review_comment|add_review_to_pr|compare_commits|convert_pull_request_to_draft|create_blob|create_branch|create_commit|create_file|create_issue|create_tree|delete_file|dismiss_pull_request_review|download_user_content|download_workflow_artifact|fetch|fetch_blob|fetch_commit|fetch_commit_workflow_runs|fetch_file|fetch_issue|fetch_issue_comments|fetch_pr|fetch_pr_comments|fetch_pr_file_patch|fetch_pr_patch|fetch_workflow_job_logs|fetch_workflow_job_steps|fetch_workflow_run_artifacts|fetch_workflow_run_jobs|get_commit_combined_status|get_issue_comment_reactions|get_pr_diff|get_pr_info|get_pr_reactions|get_pr_review_comment_reactions|get_profile|get_repo|get_repo_collaborator_permission|get_user_login|get_users_recent_prs_in_repo|label_pr|list_installations|list_installed_accounts|list_pr_changed_filenames|list_pull_request_review_threads|list_pull_request_reviews|list_recent_issues|list_repositories|list_repositories_by_affiliation|list_repositories_by_installation|list_user_org_memberships|list_user_orgs|lock_issue_conversation|mark_pull_request_ready_for_review|remove_issue_assignees|remove_issue_label|remove_pull_request_reviewers|remove_reaction_from_issue_comment|remove_reaction_from_pr|remove_reaction_from_pr_review_comment|reply_to_review_comment|request_pull_request_reviewers|rerun_failed_workflow_run_jobs|rerun_workflow_job|resolve_review_thread|search|search_branches|search_commits|search_installed_repositories_streaming|search_installed_repositories_v2|search_issues|search_prs|search_repositories|unlock_issue_conversation|unresolve_review_thread|update_file|update_issue|update_issue_comment|update_ref|update_review_comment)$";
const CONNECTOR_HOOKS: &[(&str, &str)] = &[
    (ISSUE_MATCHER, "codexy-repository-issue"),
    ("^mcp__codex_apps__github_(create|update)_pull_request$", "codexy-repository-pull-request"),
    ("^mcp__codex_apps__github_(merge_pull_request|enable_auto_merge)$", "codexy-repository-merge"),
];

#[test]
fn installed_plugin_activates_the_native_github_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let hooks = read(&root.join("hooks/hooks.json"))?;
    assert_eq!(hooks["hooks"]["UserPromptSubmit"].as_array().ok_or("prompt hooks")?.len(), 1);
    let pre_tool_use = hooks["hooks"]["PreToolUse"].as_array().ok_or("admission hooks")?;
    assert_eq!(pre_tool_use.len(), 7);
    for group in &pre_tool_use[..2] {
        let handler = &group["hooks"][0];
        assert_eq!(handler["type"], "command");
        assert_eq!(handler["timeout"], 5);
        assert!(handler["command"].as_str().unwrap_or_default().contains("${PLUGIN_ROOT}/hooks/codexy-github-admission.sh"));
        assert!(handler["commandWindows"].as_str().unwrap_or_default().contains("${PLUGIN_ROOT}/hooks/codexy-github-admission-"));
    }
    for (group, (matcher, launcher)) in pre_tool_use[2..5].iter().zip(CONNECTOR_HOOKS) {
        assert_connector_hook(group, "PreToolUse", matcher, launcher)?;
    }
    assert_generic_command_safety_hooks(&pre_tool_use[5..], "PreToolUse")?;
    let permission = hooks["hooks"]["PermissionRequest"]
        .as_array()
        .ok_or("permission hooks")?;
    assert_eq!(permission.len(), 5);
    for (group, (matcher, launcher)) in permission[..3].iter().zip(CONNECTOR_HOOKS) {
        assert_connector_hook(group, "PermissionRequest", matcher, launcher)?;
    }
    assert_generic_command_safety_hooks(&permission[3..], "PermissionRequest")?;
    let installed = hooks.to_string();
    for (_, launcher) in CONNECTOR_HOOKS {
        assert!(installed.contains(launcher));
    }
    let path = root.join("hooks/hooks.json");
    let mut missing_permission = read(&path)?;
    missing_permission["hooks"]
        .as_object_mut()
        .ok_or("hooks object")?
        .remove("PermissionRequest");
    std::fs::write(&path, serde_json::to_vec(&missing_permission)?)?;
    let invalid = validate(&root)?;
    assert!(!invalid.status.success());
    assert!(
        text(&invalid).contains("UserPromptSubmit, PermissionRequest, and PreToolUse"),
        "{}",
        text(&invalid)
    );
    Ok(())
}

fn assert_connector_hook(
    group: &serde_json::Value,
    event: &str,
    matcher: &str,
    launcher: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(group["matcher"], matcher);
    let handler = &group["hooks"][0];
    assert_eq!(handler["type"], "command");
    assert_eq!(handler["timeout"], 5);
    assert_eq!(handler["command"], format!("\"${{PLUGIN_ROOT}}/hooks/{launcher}.sh\" {event}"));
    assert_eq!(handler["commandWindows"], format!("\"${{PLUGIN_ROOT}}/hooks/{launcher}.cmd\" {event}"));
    Ok(())
}

fn assert_generic_command_safety_hooks(
    groups: &[serde_json::Value],
    event: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(groups.len(), 2);
    for (group, launcher) in groups.iter().zip([
        "codexy-repository-github-command",
        "codexy-destructive-command",
    ]) {
        let handler = &group["hooks"][0];
        assert_eq!(group["matcher"], "^Bash$");
        assert_eq!(handler["type"], "command");
        assert_eq!(handler["timeout"], 5);
        assert_eq!(
            handler["command"],
            format!("\"${{PLUGIN_ROOT}}/hooks/{launcher}.sh\" {event}")
        );
        assert_eq!(
            handler["commandWindows"],
            format!("\"${{PLUGIN_ROOT}}/hooks/{launcher}.cmd\" {event}")
        );
    }
    Ok(())
}

#[test]
fn installed_command_safety_applies_outside_codexy_without_repository_governance()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let unrelated = temp.path().join("unrelated-repository");
    std::fs::create_dir_all(unrelated.join(".git"))?;
    std::fs::create_dir_all(unrelated.join(".codex"))?;
    std::fs::write(
        unrelated.join(".git/config"),
        "[remote \"origin\"]\n\turl = https://github.com/example/noncodex.git\n",
    )?;
    std::fs::write(
        unrelated.join(".codex/repository-github-policy.json"),
        "{\"schema\":\"codexy.repository-github-policy/v1\",\"repository\":\"example/noncodex\"}",
    )?;
    assert!(
        !unrelated.join(".codex/hooks.json").exists(),
        "the unrelated repository must rely on the installed plugin"
    );
    let github_input = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "gh repo view"},
        "cwd": unrelated,
    });
    let github = run_installed_launcher(
        &root,
        "codexy-repository-github-command",
        &github_input,
    )?;
    assert!(
        github.is_empty(),
        "generic GitHub command admission denied a read-only operation"
    );
    let destructive_input = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "rm -rf /"},
        "cwd": unrelated,
    });
    let destructive = run_installed_launcher(
        &root,
        "codexy-destructive-command",
        &destructive_input,
    )?;
    assert!(
        destructive.contains("CODEXY_DESTRUCTIVE_COMMAND_DESTRUCTIVE_EFFECT"),
        "generic destructive safety was not installed: {destructive}"
    );
    Ok(())
}

fn run_installed_launcher(
    root: &Path,
    launcher: &str,
    input: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new(root.join(format!("hooks/{launcher}.sh")))
        .arg("PreToolUse")
        .env("PLUGIN_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("launcher stdin")?
        .write_all(&serde_json::to_vec(input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "{launcher}");
    assert!(output.stderr.is_empty(), "{launcher}");
    Ok(String::from_utf8(output.stdout)?)
}
