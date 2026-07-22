use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn shell_policy_models_composed_execution_context() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", &["git@github.com:eunsoogi/codexy.git"])?;
    let foreign = repository(workspace.path(), "foreign", &["https://github.com/openai/codex.git"])?;
    let multi_foreign = repository(
        workspace.path(),
        "multi-foreign",
        &["https://github.com/openai/codex.git", "https://github.com/rust-lang/cargo.git"],
    )?;
    for command in [
        "GIT_DIR=../owned/.git git -C foreign push --force origin topic",
        "GIT_DIR=../owned/.git env -C foreign git push --force origin topic",
        "bash -lc 'git push --force git@github.com:eunsoogi/codexy.git topic'",
        "eval 'git push --force' 'git@github.com:eunsoogi/codexy.git topic'",
        "GH_REPO=github.com/eunsoogi/codexy gh pr create --title 'Plain title'",
    ] {
        let output = bash(&root, workspace.path(), command)?;
        assert_deny(&output).map_err(|error| format!("{command}: {output:?}: {error}"))?;
    }
    assert_eq!(bash(&root, &foreign, "GIT_DIR=../owned/.git env -u GIT_DIR git push --force origin topic")?, b"");
    assert_eq!(bash(&root, &foreign, "env -u GH_REPO gh pr create --title 'Plain title'")?, b"");
    assert_eq!(bash(&root, &foreign, "eval 'printf %s' git@github.com:eunsoogi/codexy.git")?, b"");
    assert_eq!(bash(&root, &multi_foreign, "git reset --hard HEAD")?, b"");
    assert!(owned.exists());
    Ok(())
}

fn plugin_root() -> std::path::PathBuf { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy") }
fn bash(root: &std::path::Path, cwd: &std::path::Path, command: &str) -> TestResult<Vec<u8>> {
    let input = json!({"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":command},"cwd":cwd});
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg("PreToolUse").env_clear().env("PLUGIN_ROOT", root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(&input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
}
fn assert_deny(bytes: &[u8]) -> TestResult { assert_eq!(serde_json::from_slice::<serde_json::Value>(bytes)?["hookSpecificOutput"]["hookEventName"], "PreToolUse"); Ok(()) }
fn repository<'a>(root: &std::path::Path, name: &str, remotes: &[&'a str]) -> TestResult<std::path::PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    let config = remotes.iter().enumerate().map(|(index, remote)| format!("[remote \"remote-{index}\"]\n\turl = {remote}\n")).collect::<String>();
    std::fs::write(path.join(".git/config"), config)?;
    Ok(path)
}
