use crate::support::{FixtureCommand as Command, write_posix_fixture_command};
use super::{TestResult, assert_input, assert_tool_case, plugin_root, repository};
use serde_json::{Value, json};

#[test]
fn connector_inputs_require_owned_repository_and_reject_unknown_fields() -> TestResult {
    let root = plugin_root();
    for (tool, owned, owned_denied) in cases() {
        assert_tool_case(&root, tool, owned.clone(), owned_denied)?;

        let mut foreign = owned.clone();
        foreign["repository_full_name"] = json!("openai/codex");
        corrupt_foreign_input(tool, &mut foreign);
        assert_tool_case(&root, tool, foreign, true)?;

        let mut missing = owned.clone();
        missing.as_object_mut().ok_or("connector input")?
            .remove("repository_full_name");
        assert_tool_case(&root, tool, missing, true)?;

        let mut unknown = owned;
        unknown["unexpected"] = json!(true);
        assert_tool_case(&root, tool, unknown, true)?;

        for repository in [
            "https://github.com/eunsoogi/codexy",
            "github.com/eunsoogi/codexy",
            "https://github.com/openai/codex",
            "github.com/openai/codex",
        ] {
            let mut noncanonical = cases().into_iter()
                .find_map(|(candidate, input, _)| (candidate == tool).then_some(input))
                .ok_or("connector case")?;
            noncanonical["repository_full_name"] = json!(repository);
            assert_tool_case(&root, tool, noncanonical, true)?;
        }
    }
    let missing_cwd = cases()[0].1.clone();
    for cwd in [Value::Null, json!("relative"), json!({"not":"a path"})] {
        assert_input(
            &root,
            json!({"hook_event_name":"PreToolUse","tool_name":"mcp__codex_apps__github_create_issue","tool_input":missing_cwd.clone(),"cwd":cwd}),
            true,
            &[],
        )?;
    }
    Ok(())
}

#[test]
fn issue_735_connector_positive_matrix_is_explicit() -> TestResult {
    let root = plugin_root();
    for (case_id, tool, input) in [
        ("P-ISS-01", "github_create_issue", json!({"repository_full_name":"eunsoogi/codexy","title":"Valid issue"})),
        ("P-ISS-02", "github_update_issue", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"body":"note"})),
        ("P-ISS-03", "github_update_issue", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"state":"closed","state_reason":"completed"})),
        ("P-ISS-03-reopen", "github_update_issue", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"state":"open","state_reason":"reopened"})),
        ("P-ISS-05", "github_add_issue_labels", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"labels":["bug"]})),
        ("P-ISS-05-remove", "github_remove_issue_label", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"label":"old"})),
        ("P-ISS-05-clear", "github_update_issue", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"labels":[]})),
        ("P-ISS-06", "github_add_issue_assignees", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"assignees":["eunsoogi"]})),
        ("P-ISS-06-remove", "github_remove_issue_assignees", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"assignees":["old"]})),
        ("P-ISS-06-clear", "github_update_issue", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"assignees":[]})),
        ("P-ISS-07", "github_update_issue", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"milestone":23})),
        ("P-PR-01", "github_create_pull_request", json!({"repository_full_name":"eunsoogi/codexy","title":"fix(hooks): create safe PR","head_branch":"topic","base_branch":"main"})),
        ("P-PR-02", "github_update_pull_request", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"body":"note","maintainer_can_modify":false})),
        ("P-PR-03", "github_update_pull_request", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"state":"closed"})),
        ("P-PR-04", "github_add_comment_to_issue", json!({"repo_full_name":"eunsoogi/codexy","pr_number":17,"comment":"note"})),
        ("P-PR-05", "github_add_review_to_pr", json!({"repo_full_name":"eunsoogi/codexy","pr_number":17,"action":"APPROVE","file_comments":null})),
        ("P-PR-06", "github_request_pull_request_reviewers", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"reviewers":["eunsoogi"]})),
        ("P-PR-06-remove", "github_remove_pull_request_reviewers", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"team_reviewers":["old"]})),
        ("P-PR-07", "github_convert_pull_request_to_draft", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17})),
        ("P-PR-08", "github_mark_pull_request_ready_for_review", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17})),
    ] {
        assert_connector_case(&root, case_id, tool, input, false)?;
    }
    for (case_id, tool, input) in [
        ("N-11-connector-label", "github_label_pr", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"label":"bug"})),
        ("N-11-connector-comment", "github_update_issue_comment", json!({"repo_full_name":"eunsoogi/codexy","comment_id":7,"comment":"rewrite"})),
        ("N-11-connector-branch", "github_create_branch", json!({"repository_full_name":"eunsoogi/codexy","branch":"topic"})),
        ("N-11-empty-add-labels", "github_add_issue_labels", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"labels":[]})),
        ("N-11-empty-remove-assignees", "github_remove_issue_assignees", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"assignees":[]})),
        ("N-17-review-without-body", "github_add_review_to_pr", json!({"repo_full_name":"eunsoogi/codexy","pr_number":17,"action":"COMMENT"})),
    ] {
        assert_connector_case(&root, case_id, tool, input, true)?;
    }
    Ok(())
}

#[test]
fn issue_735_connector_reads_do_not_require_repository_governance() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = workspace.path().join("unconfigured");
    std::fs::create_dir_all(&cwd)?;
    assert_input(
        &root,
        json!({"hook_event_name":"PreToolUse","tool_name":"mcp__codex_apps__github_get_repo","tool_input":{},"cwd":cwd}),
        false,
        &[],
    )
}

#[test]
fn issue_735_unknown_connector_tools_fail_closed_at_the_universal_launcher() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_input(
            &root,
            json!({
                "hook_event_name": event,
                "tool_name": "mcp__codex_apps__github_future_remote_mutation",
                "tool_input": {"repository_full_name": "eunsoogi/codexy", "issue_number": 17},
                "cwd": cwd,
            }),
            true,
            &[],
        )?;
    }
    Ok(())
}

#[test]
fn issue_758_pr_754_unhooked_connector_payload_is_unavailable() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let fake_bin = workspace.path().join("fake-bin");
    std::fs::create_dir(&fake_bin)?;
    let recorder = workspace.path().join("merge-recorder");
    write_posix_fixture_command(
        &fake_bin.join("gh"),
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CODEXY_GH_RECORD\"\n",
    )?;
    macro_rules! post_hook_mutation {
        ($launcher:expr, $payload:expr, $args:expr) => {{
            // This fixture models hook-then-tool sequencing, never host `functions.exec`.
            let output = super::concern_launchers::run_launcher(&root, $launcher, "PreToolUse", $payload, &[])?;
            if output.is_empty() {
                let output = Command::new(fake_bin.join("gh"))
                    .env("CODEXY_GH_RECORD", &recorder)
                    .args($args)
                    .output()?;
                assert!(output.status.success());
                Ok::<Option<Vec<u8>>, Box<dyn std::error::Error>>(None)
            } else {
                Ok(Some(output))
            }
        }};
    }
    let mut admitted = json!({"hook_event_name":"PreToolUse","tool_name":"mcp__codex_apps__github_create_pull_request","cwd":cwd.clone()});
    admitted["tool_input"] = cases()[1].1.clone();
    let admitted_output = post_hook_mutation!("codexy-repository-pull-request", &admitted, ["pr", "create", "--repo", "eunsoogi/codexy", "--title", "admitted control"])?;
    assert!(admitted_output.is_none(), "{admitted_output:?}");
    assert_eq!(std::fs::read_to_string(&recorder)?, "pr\ncreate\n--repo\neunsoogi/codexy\n--title\nadmitted control\n");
    std::fs::remove_file(&recorder)?;
    let input = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"mcp__codex_apps__github_merge_pull_request",
        "tool_input":{
            "repository_full_name":"eunsoogi/codexy",
            "pr_number":754,
            "merge_method":"squash",
            "expected_head_sha":"b3fb207819c8246c0dbea33f6a3dea7ecfab93e9",
            "commit_title":"fix(hooks): narrow shell policy false positives",
            "commit_message":"rewritten body\n\nFixes #736",
            "authorization_comment":"AUTHORIZE SQUASH MERGE: PR #754\nBASE: main@6ac81bc5b34ec5af0094a4ad9ff361bbbb1c3dba\nHEAD: b3fb207819c8246c0dbea33f6a3dea7ecfab93e9"
        },
        "cwd":cwd,
    });
    let output = post_hook_mutation!("codexy-repository-merge", &input, ["pr", "merge", "754", "--repo", "eunsoogi/codexy"])?.ok_or("merge hook admitted")?;
    let denial: Value = serde_json::from_slice(&output)?;
    assert_eq!(denial["hookSpecificOutput"]["permissionDecision"], "deny");
    let reason = denial["hookSpecificOutput"]["permissionDecisionReason"].as_str().unwrap_or_default();
    assert!(reason.contains("UNAVAILABLE"), "{reason}");
    assert!(!recorder.exists(), "nested connector reached fake GitHub merge");
    Ok(())
}

fn cases() -> [(&'static str, Value, bool); 4] {
    [
        (
            "mcp__codex_apps__github_create_issue",
            json!({
                "repository_full_name": "eunsoogi/codexy",
                "title": "Require typed connector ownership",
                "body": "## Problem\n\n## Scope\n\n## Acceptance Criteria\n\n## Verification"
            }),
            false,
        ),
        (
            "mcp__codex_apps__github_create_pull_request",
            json!({
                "repository_full_name": "eunsoogi/codexy",
                "title": "fix(hooks): require connector ownership",
                "head_branch": "topic",
                "base_branch": "main",
                "body": "## Summary\n\n## Rationale\n\n## Changed Areas\n\n## Verification\n\n## Evidence\n\n## Not Run\n\n## Follow-ups"
            }),
            false,
        ),
        (
            "mcp__codex_apps__github_merge_pull_request",
            json!({
                "repository_full_name": "eunsoogi/codexy",
                "pr_number": 551,
                "merge_method": "squash",
                "expected_head_sha": "7ebeea26367a9453e135ad495bdfc7990fc283f3",
                "commit_title": "fix(hooks): require connector ownership (#551)",
                "commit_message": "Fixes #551"
            }),
            true,
        ),
        (
            "mcp__codex_apps__github_enable_auto_merge",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":551}),
            true,
        ),
    ]
}

fn assert_connector_case(
    root: &std::path::Path,
    case_id: &str,
    tool: &str,
    input: Value,
    denied: bool,
) -> TestResult {
    let workspace = tempfile::tempdir()?;
    let cwd = super::repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_input(
            root,
            json!({"hook_event_name":event,"tool_name":format!("mcp__codex_apps__{tool}"),"tool_input":input.clone(),"cwd":cwd}),
            denied,
            &[],
        )
        .map_err(|error| format!("{case_id} {event}: {error}"))?;
    }
    Ok(())
}

fn corrupt_foreign_input(tool: &str, input: &mut Value) {
    if tool.ends_with("create_issue") || tool.ends_with("create_pull_request") {
        input["title"] = Value::Null;
    }
}
