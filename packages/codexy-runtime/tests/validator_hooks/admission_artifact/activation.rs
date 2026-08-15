use std::io::Write as _;
use std::path::Path;
use std::process::Stdio;

use crate::support::FixtureCommand as Command;

use super::super::{copy_github as copy, read, text, validate};

#[test]
fn installed_plugin_activates_the_native_github_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let hooks = read(&root.join("hooks/hooks.json"))?;
    assert_eq!(hooks["hooks"]["UserPromptSubmit"].as_array().ok_or("prompt hooks")?.len(), 1);
    let pre_tool_use = hooks["hooks"]["PreToolUse"].as_array().ok_or("admission hooks")?;
    assert_eq!(pre_tool_use.len(), 4);
    for group in &pre_tool_use[..2] {
        let handler = &group["hooks"][0];
        assert_eq!(handler["type"], "command");
        assert_eq!(handler["timeout"], 5);
        assert!(handler["command"].as_str().unwrap_or_default().contains("${PLUGIN_ROOT}/hooks/codexy-github-admission.sh"));
        assert!(handler["commandWindows"].as_str().unwrap_or_default().contains("${PLUGIN_ROOT}/hooks/codexy-github-admission-"));
    }
    assert_generic_command_safety_hooks(&pre_tool_use[2..], "PreToolUse")?;
    let permission = hooks["hooks"]["PermissionRequest"]
        .as_array()
        .ok_or("permission hooks")?;
    assert_generic_command_safety_hooks(permission, "PermissionRequest")?;
    let installed = hooks.to_string();
    for repository_only in [
        "codexy-repository-issue",
        "codexy-repository-pull-request",
        "codexy-repository-merge",
    ] {
        assert!(
            !installed.contains(repository_only),
            "repository-only governance leaked into installed hooks: {repository_only}"
        );
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
