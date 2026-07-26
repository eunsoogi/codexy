use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn opaque_graphql_mutations_fail_closed_without_blocking_queries() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(
        workspace.path(),
        "foreign",
        "https://github.com/openai/codex.git",
    )?;

    assert_case(
        &root,
        &foreign,
        "gh api graphql -f query='mutation { mergePullRequest(input:{pullRequestId:\"PR_owned_node\",mergeMethod:MERGE}) { pullRequest { id } } }'",
        true,
        &[],
    )?;
    assert_case(
        &root,
        &foreign,
        "gh api graphql -f query='query { viewer { login } }'",
        false,
        &[],
    )
}

#[test]
fn hash_path_aliases_cannot_disguise_git_mutations() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;

    assert_case(
        &root,
        &owned,
        "hash -p /usr/bin/git safe; safe push --force origin topic",
        true,
        &[],
    )?;
    assert_case(&root, &owned, "hash -t git; git status --short", false, &[])
}

#[test]
fn inherited_git_common_dir_fails_closed_for_mutations_only() -> TestResult {
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
    let git_dir = foreign.join(".git");
    let common_dir = owned.join(".git");
    let environment = [
        ("GIT_DIR", git_dir.as_os_str()),
        ("GIT_COMMON_DIR", common_dir.as_os_str()),
    ];

    assert_case(
        &root,
        &foreign,
        "git push --force origin topic",
        true,
        &environment,
    )?;
    assert_case(
        &root,
        &foreign,
        "git status --short",
        false,
        &environment,
    )
}

fn assert_case(
    root: &Path,
    cwd: &Path,
    command: &str,
    denied: bool,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult {
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
        .envs(environment.iter().copied())
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
    assert!(
        output.status.success(),
        "launcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if denied {
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("expected deny for {command:?}: {error}"))?;
        assert_eq!(
            value["hookSpecificOutput"]["permissionDecision"],
            "deny",
            "{command}"
        );
    } else {
        assert_eq!(output.stdout, b"", "{command}");
    }
    Ok(())
}

fn repository(root: &Path, name: &str, remote: &str) -> TestResult<PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    std::fs::write(
        path.join(".git/config"),
        format!("[remote \"origin\"]\n\turl = {remote}\n"),
    )?;
    Ok(path)
}

fn plugin_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
}
