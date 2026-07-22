use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn mutating_gh_api_for_the_owned_repository_is_denied() -> TestResult {
    let root = plugin_root();
    let foreign = tempfile::tempdir()?;

    assert_deny(&bash(
        &root,
        foreign.path(),
        "gh api --method PUT repos/eunsoogi/codexy/pulls/453/merge -f merge_method=merge",
    )?)?;
    assert_eq!(
        bash(
            &root,
            foreign.path(),
            "gh api --method GET repos/eunsoogi/codexy/pulls/453",
        )?,
        b""
    );
    Ok(())
}

#[test]
fn forced_git_send_pack_for_the_owned_repository_is_denied() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;
    let foreign = repository(
        workspace.path(),
        "foreign",
        "https://github.com/openai/codex.git",
    )?;

    assert_deny(&bash(
        &root,
        &owned,
        "git send-pack --force origin refs/heads/topic:refs/heads/topic",
    )?)?;
    assert_eq!(
        bash(
            &root,
            &owned,
            "git send-pack origin refs/heads/topic:refs/heads/topic",
        )?,
        b""
    );
    assert_eq!(
        bash(
            &root,
            &foreign,
            "git send-pack --force origin refs/heads/topic:refs/heads/topic",
        )?,
        b""
    );
    Ok(())
}

fn assert_deny(bytes: &[u8]) -> TestResult {
    let value: Value = serde_json::from_slice(bytes)?;
    assert_eq!(
        value["hookSpecificOutput"]["permissionDecision"],
        "deny"
    );
    Ok(())
}

fn bash(root: &std::path::Path, cwd: &std::path::Path, command: &str) -> TestResult<Vec<u8>> {
    let payload = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command},
        "cwd": cwd,
    });
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child
        .arg("PreToolUse")
        .env_clear()
        .env("PLUGIN_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin")?
        .write_all(&serde_json::to_vec(&payload)?)?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "launcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

fn repository(
    root: &std::path::Path,
    name: &str,
    remote: &str,
) -> TestResult<std::path::PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    std::fs::write(
        path.join(".git/config"),
        format!("[remote \"origin\"]\n\turl = {remote}\n"),
    )?;
    Ok(path)
}

fn plugin_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
}
