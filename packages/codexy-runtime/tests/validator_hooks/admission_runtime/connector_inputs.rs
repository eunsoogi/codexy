use super::{TestResult, assert_input, assert_tool_case, plugin_root};
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
fn issue_735_connector_operations_map_to_one_closed_matrix_row() -> TestResult {
    let root = plugin_root();
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_create_issue",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "title": "Valid issue"
        }),
        false,
    )?;
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_update_issue",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "issue_number": 17,
            "labels": ["bug"]
        }),
        false,
    )?;
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_update_issue",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "issue_number": 17
        }),
        true,
    )?;
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_update_issue",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "issue_number": 17,
            "title": "Updated issue",
            "labels": ["bug"]
        }),
        true,
    )?;
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_update_issue",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "issue_number": 17,
            "state": "closed",
            "state_reason": "completed"
        }),
        false,
    )?;
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_update_issue",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "issue_number": 17,
            "state": "closed",
            "state_reason": "duplicate"
        }),
        true,
    )?;
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_create_pull_request",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "title": "fix(hooks): create safe PR",
            "head_branch": "topic",
            "base_branch": "main"
        }),
        false,
    )?;
    assert_tool_case(
        &root,
        "mcp__codex_apps__github_update_pull_request",
        json!({
            "repository_full_name": "eunsoogi/codexy",
            "pr_number": 17,
            "state": "closed"
        }),
        false,
    )?;
    Ok(())
}

#[test]
fn issue_735_connector_positive_matrix_is_explicit() -> TestResult {
    let root = plugin_root();
    for (tool, input) in [
        ("github_add_comment_to_issue", json!({"repo_full_name":"eunsoogi/codexy","pr_number":17,"comment":"note"})),
        ("github_add_issue_assignees", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"assignees":["eunsoogi"]})),
        ("github_remove_issue_assignees", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"assignees":["old"]})),
        ("github_add_issue_labels", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"labels":["bug"]})),
        ("github_remove_issue_label", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"label":"old"})),
        ("github_add_review_to_pr", json!({"repo_full_name":"eunsoogi/codexy","pr_number":17,"action":"APPROVE","file_comments":null})),
        ("github_request_pull_request_reviewers", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"reviewers":["eunsoogi"]})),
        ("github_remove_pull_request_reviewers", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"team_reviewers":["old"]})),
        ("github_convert_pull_request_to_draft", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17})),
        ("github_mark_pull_request_ready_for_review", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17})),
    ] {
        assert_tool_case(&root, &format!("mcp__codex_apps__{tool}"), input, false)?;
    }
    assert_tool_case(&root, "mcp__codex_apps__github_update_issue", json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"body":null}), false)?;
    assert_tool_case(&root, "mcp__codex_apps__github_update_pull_request", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"maintainer_can_modify":false}), false)?;
    for (tool, input) in [
        ("github_label_pr", json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"label":"bug"})),
        ("github_update_issue_comment", json!({"repo_full_name":"eunsoogi/codexy","comment_id":7,"comment":"rewrite"})),
        ("github_create_branch", json!({"repository_full_name":"eunsoogi/codexy","branch":"topic"})),
    ] {
        assert_tool_case(&root, &format!("mcp__codex_apps__{tool}"), input, true)?;
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

fn cases() -> [(&'static str, Value, bool); 3] {
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
    ]
}

fn corrupt_foreign_input(tool: &str, input: &mut Value) {
    if tool.ends_with("create_issue") || tool.ends_with("create_pull_request") {
        input["title"] = Value::Null;
    }
}
