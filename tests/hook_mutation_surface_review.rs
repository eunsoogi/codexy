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

#[test]
fn backslash_continued_git_push_is_denied() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;

    assert_deny(&bash(
        &root,
        &owned,
        concat!("git pu", "\\\n", "sh --force origin topic"),
    )?)
}

#[test]
fn coprocess_mutations_are_denied() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;

    assert_deny(&bash(&root, &owned, "coproc git push --force origin topic")?)?;
    assert_deny(&bash(&root, &owned, "coproc gh pr merge 453 --merge")?)?;
    assert_eq!(bash(&root, &owned, "coproc printf safe")?, b"");
    Ok(())
}

#[test]
fn included_owned_remote_is_part_of_the_effective_push_target() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let included = workspace.path().join("included.gitconfig");
    std::fs::write(
        &included,
        "[remote \"origin\"]\n\turl = git@github.com:eunsoogi/codexy.git\n",
    )?;
    let mixed = workspace.path().join("mixed");
    std::fs::create_dir_all(&mixed)?;
    let initialized = Command::new("git").args(["-C", mixed.to_str().ok_or("mixed path")?, "init", "-q"]).status()?;
    assert!(initialized.success(), "git init failed");
    std::fs::write(
        mixed.join(".git/config"),
        format!(
            "[include]\n\tpath = {}\n[remote \"origin\"]\n\turl = https://github.com/openai/codex.git\n",
            included.display()
        ),
    )?;

    assert_deny(&bash(&root, &mixed, "git push --force origin topic")?)?;
    assert_eq!(bash(&root, &mixed, "git push origin topic")?, b"");
    Ok(())
}

#[test]
fn github_api_current_repository_placeholders_are_denied() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;

    assert_deny(&bash(
        &root,
        &owned,
        "gh api --method PUT 'repos/{owner}/{repo}/pulls/453/merge' -f merge_method=merge",
    )?)?;
    assert_eq!(
        bash(&root, &owned, "gh api --method GET 'repos/{owner}/{repo}/pulls/453'")?,
        b""
    );
    Ok(())
}

#[test]
fn configured_github_alias_is_expanded_before_admission() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let home = tempfile::tempdir()?;
    let owned = repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;
    std::fs::create_dir_all(home.path().join(".config/gh"))?;
    std::fs::write(
        home.path().join(".config/gh/config.yml"),
        "aliases:\n    land: pr merge 453 --merge\n    inspect: pr view 453\n",
    )?;

    assert_deny(&bash_with_home(&root, &owned, "gh land", Some(home.path()))?)?;
    assert_eq!(bash_with_home(&root, &owned, "gh inspect", Some(home.path()))?, b"");
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
    bash_with_home(root, cwd, command, None)
}

fn bash_with_home(
    root: &std::path::Path,
    cwd: &std::path::Path,
    command: &str,
    home: Option<&std::path::Path>,
) -> TestResult<Vec<u8>> {
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
    if let Some(home) = home {
        child.env("HOME", home);
        if let Some(path) = std::env::var_os("PATH") {
            child.env("PATH", path);
        }
    }
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
