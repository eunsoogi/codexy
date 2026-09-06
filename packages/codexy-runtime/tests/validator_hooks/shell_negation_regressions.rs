use super::admission_runtime::{TestResult, assert_event_case, plugin_root, repository};

#[test]
fn negated_connector_branches_preserve_credential_policy() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let cases = [
        ("! true || GH_TOKEN=fixture gh issue list", true),
        ("! false && GH_TOKEN=fixture gh issue list", true),
        ("! false || GH_TOKEN=fixture gh issue list", false),
        ("! true && GH_TOKEN=fixture gh issue list", false),
        ("! ! true || GH_TOKEN=fixture gh issue list", false),
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for (command, denied) in cases {
            assert_event_case(&root, event, &owned, command, denied, &[])?;
        }
    }
    Ok(())
}
