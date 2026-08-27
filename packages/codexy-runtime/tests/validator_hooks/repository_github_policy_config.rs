use crate::support::FixtureCommand as Command;

#[test]
fn repository_github_policy_configuration_is_strict_and_complete() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    write_config(root)?;
    let script = codexy_runtime::paths::repository_root().join("scripts/validate-repository-github-policy");
    let valid = Command::new(&script).args(["--root", root.to_str().ok_or("root")?]).output()?;
    assert!(valid.status.success(), "{}", String::from_utf8_lossy(&valid.stderr));

    std::fs::write(root.join(".codex/repository-github-policy.json"), "{\"schema\":\"codexy.repository-github-policy/v1\",\"schema\":\"duplicate\",\"repository\":\"eunsoogi/codexy\"}")?;
    let invalid = Command::new(&script).args(["--root", root.to_str().ok_or("root")?]).output()?;
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("duplicate"));
    write_config(root)?;
    let hooks = root.join(".codex/hooks.json");
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&hooks)?)?;
    value["hooks"]["PreToolUse"][0]["hooks"][0]
        .as_object_mut()
        .ok_or("handler")?
        .remove("commandWindows");
    std::fs::write(&hooks, serde_json::to_vec(&value)?)?;
    let missing_windows = Command::new(&script).args(["--root", root.to_str().ok_or("root")?]).output()?;
    assert!(!missing_windows.status.success());
    Ok(())
}

fn write_config(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let codex = root.join(".codex");
    std::fs::create_dir_all(&codex)?;
    std::fs::write(codex.join("repository-github-policy.json"), "{\"schema\":\"codexy.repository-github-policy/v1\",\"repository\":\"eunsoogi/codexy\"}")?;
    let groups = [
        ("^mcp__codex_apps__github_(?:add_comment_to_issue|add_issue_assignees|add_issue_labels|add_reaction_to_issue_comment|add_reaction_to_pr|add_reaction_to_pr_review_comment|add_review_to_pr|compare_commits|convert_pull_request_to_draft|create_blob|create_branch|create_commit|create_file|create_issue|create_pull_request|create_tree|delete_file|dismiss_pull_request_review|download_user_content|download_workflow_artifact|enable_auto_merge|fetch|fetch_blob|fetch_commit|fetch_commit_workflow_runs|fetch_file|fetch_issue|fetch_issue_comments|fetch_pr|fetch_pr_comments|fetch_pr_file_patch|fetch_pr_patch|fetch_workflow_job_logs|fetch_workflow_job_steps|fetch_workflow_run_artifacts|fetch_workflow_run_jobs|get_commit_combined_status|get_issue_comment_reactions|get_pr_diff|get_pr_info|get_pr_reactions|get_pr_review_comment_reactions|get_profile|get_repo|get_repo_collaborator_permission|get_user_login|get_users_recent_prs_in_repo|label_pr|list_installations|list_installed_accounts|list_pr_changed_filenames|list_pull_request_review_threads|list_pull_request_reviews|list_recent_issues|list_repositories|list_repositories_by_affiliation|list_repositories_by_installation|list_user_org_memberships|list_user_orgs|lock_issue_conversation|mark_pull_request_ready_for_review|merge_pull_request|remove_issue_assignees|remove_issue_label|remove_pull_request_reviewers|remove_reaction_from_issue_comment|remove_reaction_from_pr|remove_reaction_from_pr_review_comment|reply_to_review_comment|request_pull_request_reviewers|rerun_failed_workflow_run_jobs|rerun_workflow_job|resolve_review_thread|search|search_branches|search_commits|search_installed_repositories_streaming|search_installed_repositories_v2|search_issues|search_prs|search_repositories|unlock_issue_conversation|unresolve_review_thread|update_file|update_issue|update_issue_comment|update_pull_request|update_ref|update_review_comment)$", "codexy-repository-issue"),
    ];
    let event = |name: &str| groups.iter().map(|(matcher, launcher)| serde_json::json!({"matcher":matcher,"hooks":[{"type":"command","command":format!("\"$(git rev-parse --show-toplevel)/plugins/codexy-github/hooks/{launcher}.sh\" {name}"),"commandWindows":format!("\"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" -NoLogo -NoProfile -NonInteractive -Command \"$git = Join-Path $env:ProgramFiles 'Git\\cmd\\git.exe'; if (-not (Test-Path -LiteralPath $git)) {{ exit 1 }}; $root = & $git rev-parse --show-toplevel; if ($LASTEXITCODE -ne 0 -or -not $root) {{ exit 1 }}; & (Join-Path $root 'plugins/codexy-github/hooks/{launcher}.cmd') {name}; exit $LASTEXITCODE\""),"timeout":5}]})).collect::<Vec<_>>();
    let hooks = serde_json::json!({"description":"Codexy repository GitHub governance hooks.","hooks":{"PermissionRequest":event("PermissionRequest"),"PreToolUse":event("PreToolUse")}});
    std::fs::write(codex.join("hooks.json"), serde_json::to_vec(&hooks)?)?;
    Ok(())
}
