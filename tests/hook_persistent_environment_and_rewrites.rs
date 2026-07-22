use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn persistent_environment_state_tracks_shell_scope() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let git_dir = format!("{}/.git", owned.display());

    for command in [
        format!("export GIT_DIR='{git_dir}'; git push --force origin topic"),
        format!("GIT_DIR='{git_dir}'; export GIT_DIR; git push --force origin topic"),
        format!("{{ export GIT_DIR='{git_dir}'; }}; git push --force origin topic"),
        format!("export GIT_DIR=\"{git_dir}\" && git push --force origin topic"),
    ] {
        assert_case(&root, &foreign, &command, true)?;
    }
    for command in [
        format!("export GIT_DIR='{git_dir}'; unset GIT_DIR; git push --force origin topic"),
        format!("(export GIT_DIR='{git_dir}'); git push --force origin topic"),
        format!("GIT_DIR='{git_dir}' git status; git push --force origin topic"),
        "export SAFE=value; printf '%s' \"$SAFE\"".into(),
        "unset SAFE; printf safe".into(),
    ] {
        assert_case(&root, &foreign, &command, false)?;
    }
    for command in [
        "export GIT_DIR=$UNKNOWN; git status",
        "export -Z GIT_DIR; git status",
        "unset -Z GIT_DIR; git status",
        "export 1INVALID=value; git status",
    ] {
        assert_case(&root, &foreign, command, true)?;
    }
    Ok(())
}

#[test]
fn inherited_git_dir_selects_owned_repository() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let git_dir = owned.join(".git");

    assert_case_with_context(
        &root,
        &foreign,
        "git push --force origin topic",
        true,
        None,
        None,
        Some(&git_dir),
        &[],
    )
}

#[test]
fn clone_outside_a_checkout_remains_admitted() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let outside = workspace.path().join("outside");
    std::fs::create_dir(&outside)?;

    assert_case(&root, &outside, "git clone https://github.com/openai/codex.git clone-target", false)
}

#[test]
fn remote_add_updates_sequential_force_push_admission() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;

    assert_case(
        &root,
        &foreign,
        "git remote add owned git@github.com:eunsoogi/codexy.git && git push --force owned topic",
        true,
    )
}

#[test]
fn sequential_remote_url_mutations_update_push_admission() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;

    assert_case(
        &root,
        &foreign,
        "git remote set-url origin git@github.com:eunsoogi/codexy.git && git push --force origin topic",
        true,
    )?;
    assert_case(
        &root,
        &foreign,
        "git remote set-url origin https://github.com/openai/codex.git && git push --force origin topic",
        false,
    )
}

#[test]
fn command_scoped_url_rewrites_use_git_longest_match_semantics() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://example.com/eunsoogi/codexy.git")?;
    for command in [
        "git -c 'url.git@github.com:.insteadOf=https://example.com/' push --force origin topic",
        "git -c 'URL.git@github.com:.INSTEADOF=https://example.com/' push --force origin topic",
        "git -c 'url.git@github.com:eunsoogi/.pushInsteadOf=https://example.com/eunsoogi/' push --force origin topic",
        "git -c 'url.git@github.com:wrong/.insteadOf=https://example.com/' -c 'url.git@github.com:eunsoogi/.insteadOf=https://example.com/eunsoogi/' push --force origin topic",
    ] {
        assert_case(&root, &foreign, command, true)?;
    }
    for command in [
        "git -c 'url.git@github.com:openai/.insteadOf=https://example.com/eunsoogi/' push --force origin topic",
        "git -c 'url.git@github.com:eunsoogi/.insteadOf=https://unrelated.example/' push --force origin topic",
        "git -c 'color.ui=always' push --force origin topic",
        "git -c 'url.git@github.com:eunsoogi/.insteadOf=https://example.com/eunsoogi/' push origin topic",
    ] {
        assert_case(&root, &foreign, command, false)?;
    }
    for command in [
        "git -c 'url..insteadOf=https://example.com/' push --force origin topic",
        "git -c 'url.git@github.com:eunsoogi/.insteadOf' push --force origin topic",
    ] {
        assert_case(&root, &foreign, command, true)?;
    }
    Ok(())
}

#[test]
fn effective_admission_preserves_supported_shell_and_git_selectors() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(
        workspace.path(),
        "foreign",
        "https://mirror.invalid/eunsoogi/codexy.git",
    )?;

    assert_case(
        &root,
        &foreign,
        "gh api -X PUT -H 'Accept: application/vnd.github+json' repos/eunsoogi/codexy/pulls/453/merge -f merge_method=merge",
        true,
    )?;
    assert_case(&root, &foreign, "gh api -X GET -H 'Accept: application/vnd.github+json' repos/eunsoogi/codexy/pulls/453", false)?;
    assert_case(&root, &foreign, &format!("pushd '{}' && git push --force origin topic", owned.display()), true)?;
    assert_case(&root, &foreign, &format!("pushd '{}' && git push origin topic", owned.display()), false)?;

    let git_config = [
        ("GIT_CONFIG_COUNT", "1"),
        ("GIT_CONFIG_KEY_0", "url.git@github.com:.insteadOf"),
        ("GIT_CONFIG_VALUE_0", "https://mirror.invalid/"),
    ];
    assert_case_with_context(&root, &foreign, "git push --force origin topic", true, None, None, None, &git_config)?;
    assert_case_with_context(&root, &foreign, "git push origin topic", false, None, None, None, &git_config)
}

#[test]
fn effective_shell_invocation_and_repository_context_reaches_admission() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let mirror = repository(workspace.path(), "mirror", "https://mirror.invalid/eunsoogi/codexy.git")?;
    let sourced = workspace.path().join("enter-owned.sh");
    std::fs::write(&sourced, format!("cd '{}'\n", owned.display()))?;

    assert_case(&root, &foreign, &format!("cd '{}' >/dev/null && git push --force origin topic", owned.display()), true)?;
    assert_case(&root, &foreign, &format!("cd '{}' && git push --force origin topic", owned.display()), true)?;
    assert_case(&root, &foreign, &format!("source '{}' && git push --force origin topic", sourced.display()), true)?;
    assert_case(&root, &foreign, &format!(". '{}' && git push --force origin topic", sourced.display()), true)?;
    assert_case(&root, &foreign, "if true; then echo ok; fi", false)?;
    assert_case(&root, &owned, "if true; then echo ok; fi", true)?;
    assert_case_with_context(&root, &foreign, "gh pr merge 453 --merge", true, None, Some("eunsoogi/codexy"), None, &[])?;
    assert_case(&root, &foreign, "gh pr merge 453 --merge", false)?;

    let home = workspace.path().join("home");
    std::fs::create_dir(&home)?;
    std::fs::write(home.join(".gitconfig"), "[url \"git@github.com:\"]\n\tinsteadOf = https://mirror.invalid/\n")?;
    assert_case_with_context(&root, &mirror, "git push --force origin topic", true, Some(&home), None, None, &[])?;
    assert_case(&root, &mirror, "git push --force origin topic", false)
}

fn assert_case(root: &std::path::Path, cwd: &std::path::Path, command: &str, denied: bool) -> TestResult {
    assert_case_with_context(root, cwd, command, denied, None, None, None, &[])
}

fn assert_case_with_context(
    root: &std::path::Path,
    cwd: &std::path::Path,
    command: &str,
    denied: bool,
    home: Option<&std::path::Path>,
    gh_repo: Option<&str>,
    git_dir: Option<&std::path::Path>,
    environment: &[(&str, &str)],
) -> TestResult {
    let input = json!({"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":command},"cwd":cwd});
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg("PreToolUse").env_clear().env("PLUGIN_ROOT", root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(home) = home { child.env("HOME", home); }
    if let Some(gh_repo) = gh_repo { child.env("GH_REPO", gh_repo); }
    if let Some(git_dir) = git_dir { child.env("GIT_DIR", git_dir); }
    child.envs(environment.iter().copied());
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(&input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    if denied {
        let value: Value = serde_json::from_slice(&output.stdout).map_err(|error| format!("expected deny for {command:?}: {error}"))?;
        assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny", "{command}");
    } else {
        assert_eq!(output.stdout, b"", "{command}");
    }
    Ok(())
}

fn repository(root: &std::path::Path, name: &str, remote: &str) -> TestResult<std::path::PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    std::fs::write(path.join(".git/config"), format!("[remote \"origin\"]\n\turl = {remote}\n"))?;
    Ok(path)
}

fn plugin_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
}
