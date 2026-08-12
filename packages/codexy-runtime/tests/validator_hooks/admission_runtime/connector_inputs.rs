use super::{TestResult, assert_tool_case, plugin_root};
use serde_json::{Value, json};

#[test]
fn connector_inputs_require_owned_repository_and_reject_unknown_fields() -> TestResult {
    let root = plugin_root();
    for (tool, owned, owned_denied) in cases() {
        assert_tool_case(&root, tool, owned.clone(), owned_denied)?;

        let mut foreign = owned.clone();
        foreign["repository_full_name"] = json!("openai/codex");
        corrupt_foreign_input(tool, &mut foreign);
        assert_tool_case(&root, tool, foreign, false)?;

        let mut missing = owned.clone();
        missing.as_object_mut().ok_or("connector input")?
            .remove("repository_full_name");
        assert_tool_case(&root, tool, missing, true)?;

        let mut unknown = owned;
        unknown["unexpected"] = json!(true);
        assert_tool_case(&root, tool, unknown, true)?;
    }
    Ok(())
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
