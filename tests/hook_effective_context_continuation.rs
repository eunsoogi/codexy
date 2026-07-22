use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn runtime_argument_construction_fails_closed_without_blocking_no_command_controls() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let argument_file = owned.join("force-push.args");
    std::fs::write(&argument_file, "push --force origin topic\n")?;
    assert_case(
        &root,
        &owned,
        "printf '%s\\n' 'push --force origin topic' | xargs git",
        true,
    )?;
    assert_case(
        &root,
        &owned,
        &format!("xargs --arg-file '{}' git", argument_file.display()),
        true,
    )?;
    assert_case(&root, &owned, "xargs -r git status", true)?;
    assert_case(&root, &owned, "xargs --arg-file", true)?;
    assert_case(&root, &owned, "xargs --help", false)?;
    assert_case(&root, &owned, "xargs --version", false)?;
    assert_case(&root, &owned, "xargs", false)
}

#[test]
fn pull_request_selectors_determine_the_typed_repository_target() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for command in [
        "gh pr merge https://github.com/eunsoogi/codexy/pull/453 --merge",
        "gh pr edit https://github.com/eunsoogi/codexy/pull/453 --title 'Plain title'",
        "gh pr merge https://github.com/eunsoogi/codexy/pull/not-a-number --merge",
        "gh pr merge https://github.com/eunsoogi/codexy/issues/453 --merge",
    ] {
        assert_case(&root, &foreign, command, true)?;
    }
    for command in [
        "gh pr merge https://github.com/eunsoogi/codexy/pull/453 --squash",
        "gh pr merge https://github.com/openai/codex/pull/453 --merge",
        "gh pr edit https://github.com/openai/codex/pull/453 --title 'Plain title'",
        "gh pr merge 453 --merge",
    ] {
        assert_case(&root, &foreign, command, false)?;
    }
    Ok(())
}

#[test]
fn shell_alias_recursion_preserves_normalized_git_context() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(
        &root,
        &foreign,
        &format!(
            "git -C '{}' -c alias.ship='!git push --force origin topic' ship",
            owned.display()
        ),
        true,
    )?;
    assert_case(
        &root,
        &foreign,
        &format!(
            "git --git-dir='{}/.git' -c alias.ship='!git push --force origin topic' ship",
            owned.display()
        ),
        true,
    )?;
    assert_case(
        &root,
        &foreign,
        &format!("git -C '{}' -c alias.inspect='!git status' inspect", owned.display()),
        false,
    )?;
    assert_case(
        &root,
        &owned,
        &format!(
            "git -C '{}' -c alias.ship='!git push --force origin topic' ship",
            foreign.display()
        ),
        false,
    )
}

#[test]
fn sequential_shell_state_tracks_exact_supported_mutations() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;

    for command in [
        format!("command cd '{}' && git push --force origin topic", owned.display()),
        "git config remote.origin.url git@github.com:eunsoogi/codexy.git && git push --force origin topic".into(),
        "declare -x GH_REPO=eunsoogi/codexy; gh pr merge 453 --merge".into(),
    ] {
        assert_case(&root, &foreign, &command, true)?;
    }
    for command in [
        format!("command cd '{}' && git push --force origin topic", foreign.display()),
        "git config remote.origin.url https://github.com/openai/codex.git && git push --force origin topic".into(),
        "declare -x GH_REPO=openai/codex; gh pr merge 453 --merge".into(),
    ] {
        assert_case(&root, &foreign, &command, false)?;
    }
    Ok(())
}

#[test]
fn control_flow_expansion_and_pushurl_state_close_only_sensitive_gaps() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;

    assert_case(
        &root,
        &foreign,
        &format!("C=cd; G=git; if true; then $C {}; $G push --force origin topic; fi", owned.display()),
        true,
    )?;
    assert_case(
        &root,
        &foreign,
        "git config remote.origin.pushurl git@github.com:eunsoogi/codexy.git && git push --force origin topic",
        true,
    )?;
    assert_case(&root, &foreign, "if true; then printf '%s' \"$SAFE\"; fi", false)?;
    assert_case(
        &root,
        &foreign,
        "git config remote.origin.pushurl https://github.com/openai/codex.git && git push --force origin topic",
        false,
    )
}

fn assert_case(root: &std::path::Path, cwd: &std::path::Path, command: &str, denied: bool) -> TestResult {
    let input = json!({
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
        .write_all(&serde_json::to_vec(&input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    if denied {
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny", "{command}");
    } else {
        assert_eq!(output.stdout, b"", "{command}");
    }
    Ok(())
}

fn repository(root: &std::path::Path, name: &str, remote: &str) -> TestResult<std::path::PathBuf> {
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
