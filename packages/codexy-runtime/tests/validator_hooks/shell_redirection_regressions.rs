//! Covers bounded output paths without allowing the test adapter to run commands.

use super::admission_runtime::{TestResult, assert_event_case, plugin_root, repository};
use crate::support::fixture_hook_path::modeled_path_token;
use std::path::Path;

#[test]
fn issue_1248_redirections_remain_scoped_for_both_events() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let log = shell_path(Path::new("/private/tmp/codexy-1248-run.log"))?;
    let commands = [
        (
            format!("gh run watch 37218256143 --repo eunsoogi/codexy > {log}"),
            cfg!(windows),
        ),
        ("true && gh repo view > /dev/null".to_owned(), false),
        ("gh run view 1 --repo eunsoogi/codexy --log > README.md".to_owned(), true),
        (
            "gh run view 1 --repo eunsoogi/codexy --log \"$(ln -s README.md /private/tmp/codexy-1248-prepared.log)\" > /private/tmp/codexy-1248-prepared.log".to_owned(),
            true,
        ),
        (
            "gh run view 1 --repo eunsoogi/codexy --log > '/private/tmp/<codexy-1248-marker.log'".to_owned(),
            true,
        ),
        (
            "gh run view 1 --repo eunsoogi/codexy --log > '/private/tmp/>codexy-1248-marker.log'".to_owned(),
            true,
        ),
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for (command, denied) in &commands {
            // Each command is inert adapter input; even substitutions never execute.
            assert_event_case(&root, event, &owned, command, *denied, &[])?;
        }
    }
    Ok(())
}

fn shell_path(path: &Path) -> TestResult<String> {
    modeled_path_token(path.to_str().ok_or("path")?, &|value| Ok(value.to_owned()))?
        .ok_or_else(|| "absolute shell path".into())
}
