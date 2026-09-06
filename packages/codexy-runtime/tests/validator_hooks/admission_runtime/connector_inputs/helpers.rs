use super::{TestResult, assert_input, repository};
use serde_json::{Value, json};

pub(super) fn assert_connector_case(
    root: &std::path::Path,
    case_id: &str,
    tool: &str,
    input: Value,
    denied: bool,
) -> TestResult {
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_input(
            root,
            json!({"hook_event_name":event,"tool_name":tool_name(tool),"tool_input":input.clone(),"cwd":cwd}),
            denied,
            &[],
        )
        .map_err(|error| format!("{case_id} {event}: {error}"))?;
    }
    Ok(())
}

fn tool_name(tool: &str) -> String {
    if tool.starts_with("github.") {
        tool.to_owned()
    } else {
        format!("mcp__codex_apps__{tool}")
    }
}

pub(super) fn corrupt_foreign_input(tool: &str, input: &mut Value) {
    if tool.ends_with("create_issue") || tool.ends_with("create_pull_request") {
        input["title"] = Value::Null;
    }
}
