use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn git_global_options_are_normalized_before_policy_evaluation() -> TestResult {
    let root = root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let owned = "git@github.com:eunsoogi/codexy.git";
    assert_eq!(bash(&root, workspace.path(), "git -C foreign status")?, b"");
    for command in [
        format!("git -p push --force {owned} topic"),
        format!("git -P push --force {owned} topic"),
        format!("git --paginate push --force {owned} topic"),
        format!("git --no-pager push --force {owned} topic"),
        format!("git -C foreign -c color.ui=false -p push --force {owned} topic"),
        format!("env -C foreign git -P push --force {owned} topic"),
        format!("git --work-tree foreign -p push --force {owned} topic"),
        "git --config-env=color.ui=GIT_COLOR push --force origin topic".to_owned(),
    ] {
        assert_deny(&bash(&root, foreign.as_path(), &command)?).map_err(|error| format!("{command}: {error}"))?;
    }
    assert_eq!(bash(&root, foreign.as_path(), "git -p push --force origin topic")?, b"");
    Ok(())
}

#[test]
fn effective_invocation_resolves_wrappers_and_git_aliases_before_policy() -> TestResult {
    let root = root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    append_config(&owned, "[alias]\n\tship = push --force\n\tsafe = status\n\tshell = !git push --force\n\tloop-a = loop-b\n\tloop-b = loop-a\n")?;
    append_config(&foreign, "[alias]\n\tship = push --force git@github.com:eunsoogi/codexy.git\n")?;
    for command in [
        "nice git push --force origin topic",
        "nice -n 10 git push --force origin topic",
        "git ship origin topic",
        "git -c alias.ship='push --force' ship origin topic",
        "git shell origin topic",
        "git loop-a",
    ] {
        assert_deny(&bash(&root, &owned, command)?).map_err(|error| format!("{command}: {error}"))?;
    }
    assert_deny(&bash(&root, &foreign, "git ship topic")?)?;
    assert_eq!(bash(&root, &owned, "git safe")?, b"");
    assert_eq!(bash(&root, &foreign, "nice git push --force origin topic")?, b"");
    Ok(())
}

fn bash(root: &std::path::Path, cwd: &std::path::Path, command: &str) -> TestResult<Vec<u8>> {
    let payload = json!({"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":command},"cwd":cwd});
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg("PreToolUse").env_clear().env("PLUGIN_ROOT", root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(&payload)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
}

fn assert_deny(bytes: &[u8]) -> TestResult {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_ne!(value["hookSpecificOutput"]["permissionDecisionReason"], "Codexy policy: MUST NOT execute when the static admission runtime is unavailable.");
    Ok(())
}

fn repository(root: &std::path::Path, name: &str, remote: &str) -> TestResult<std::path::PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    std::fs::write(path.join(".git/config"), format!("[remote \"origin\"]\n\turl = {remote}\n"))?;
    Ok(path)
}

fn append_config(path: &std::path::Path, text: &str) -> TestResult {
    use std::io::Write as _;
    let mut config = std::fs::OpenOptions::new().append(true).open(path.join(".git/config"))?;
    config.write_all(text.as_bytes())?;
    Ok(())
}

fn root() -> std::path::PathBuf { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy") }
