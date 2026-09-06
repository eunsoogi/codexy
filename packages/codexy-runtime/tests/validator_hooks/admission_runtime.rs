use crate::support::{FixtureCommand as Command, hook_fixture_model_input};
use serde_json::{Value, json};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn github_mutations_are_not_vetoed_by_the_retained_bash_safety_hook() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "unrelated", "https://github.com/example/project.git")?;
    let commands = [
        "gh issue create --title arbitrary",
        "gh pr create --title arbitrary --body arbitrary",
        "gh pr merge 17 --squash",
        "gh workflow run unrelated.yml --ref topic",
        "gh release create v1.0.0",
        "gh api --method POST repos/example/project/releases -f tag_name=v1",
        "gh api --method PATCH repos/example/project/issues/17 -f state=closed",
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for command in commands {
            assert_event_case(&root, event, &cwd, command, false, &[])?;
        }
    }
    Ok(())
}

#[test]
fn credential_access_remains_protected_without_a_github_mutation_classifier() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "unrelated", "https://github.com/example/project.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        for command in [
            "gh auth token",
            "gh auth status --show-token",
            "GH_TOKEN=fixture gh issue list",
            "gh api -H 'Authorization: Bearer fixture' repos/example/project",
            "gh api -H 'x-github-token: fixture' repos/example/project",
        ] {
            assert_event_case(&root, event, &cwd, command, true, &[])?;
        }
    }
    Ok(())
}

#[test]
fn local_filesystem_and_git_destructive_effects_remain_protected() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        for command in [
            "rm -rf /",
            "git reset --hard HEAD",
            "git clean -fd",
            "git push --force origin topic",
            "git add -A",
        ] {
            assert_event_case(&root, event, &cwd, command, true, &[])?;
        }
        assert_event_case(&root, event, &cwd, "git status --short", false, &[])?;
        assert_event_case(&root, event, &cwd, "gh issue create --title arbitrary", false, &[])?;
    }
    Ok(())
}

#[test]
fn opaque_or_unknown_local_effects_fail_closed_without_reintroducing_github_veto() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    assert_case(&root, &cwd, "eval '$COMMAND'", true, &[])?;
    assert_case(&root, &cwd, "sh -c '$COMMAND'", true, &[])?;
    assert_case(&root, &cwd, "if true; then gh issue create --title arbitrary; fi", false, &[])?;
    assert_case(&root, &cwd, "printf '%s\\n' 'gh issue create --title arbitrary'", false, &[])?;
    Ok(())
}

#[test]
fn malformed_hook_envelopes_fail_closed_for_the_retained_launcher() -> TestResult {
    let root = plugin_root();
    let launcher = launcher_path(&root);
    for (event, payload) in [
        ("PreToolUse", vec![0xff]),
        (
            "PreToolUse",
            br#"{"hook_event_name":"PreToolUse","hook_event_name":"PreToolUse","tool_name":"Bash"}"#.to_vec(),
        ),
        ("PermissionRequest", br#"{}"#.to_vec()),
    ] {
        let mut child = Command::new(&launcher)
            .arg(event)
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child.stdin.take().ok_or("launcher stdin")?.write_all(&payload)?;
        let output = child.wait_with_output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let denial: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(output.stdout.windows(8).any(|window| window == b"ENVELOPE"));
    }
    Ok(())
}

pub(super) fn assert_case(
    root: &Path,
    cwd: &Path,
    command: &str,
    denied: bool,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult {
    assert_event_case(root, "PreToolUse", cwd, command, denied, environment)
}

pub(super) fn assert_event_case(
    root: &Path,
    event: &str,
    cwd: &Path,
    command: &str,
    denied: bool,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult {
    let (command, cwd) = hook_fixture_model_input(command, cwd).map_err(std::io::Error::other)?;
    let input = json!({
        "hook_event_name": event,
        "tool_name": "Bash",
        "tool_input": {"command": command},
        "cwd": cwd,
    });
    let output = run_launcher(root, event, &input, environment)?;
    assert_eq!(!output.is_empty(), denied, "{event}: {command}");
    if denied {
        let value: Value = serde_json::from_slice(&output)?;
        let decision = if event == "PermissionRequest" {
            &value["hookSpecificOutput"]["decision"]["behavior"]
        } else {
            &value["hookSpecificOutput"]["permissionDecision"]
        };
        assert_eq!(decision, "deny", "missing denial for {event}: {command}");
    }
    Ok(())
}

pub(super) fn run_launcher(
    root: &Path,
    event: &str,
    input: &Value,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult<Vec<u8>> {
    let mut child = Command::new(launcher_path(root))
        .arg(event)
        .env("PLUGIN_ROOT", root)
        .envs(environment.iter().copied())
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
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    Ok(output.stdout)
}

fn launcher_path(root: &Path) -> PathBuf {
    root.join("hooks").join(if cfg!(windows) {
        "codexy-destructive-command.cmd"
    } else {
        "codexy-destructive-command.sh"
    })
}

pub(super) fn assert_event_cases(
    root: &Path,
    event: &str,
    cwd: &Path,
    cases: Vec<(String, bool)>,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult {
    for (command, denied) in cases {
        assert_event_case(root, event, cwd, &command, denied, environment)?;
    }
    Ok(())
}

pub(super) fn executable(name: &str) -> TestResult<PathBuf> {
    crate::support::executable_path(name).map_err(Into::into)
}

pub(super) fn plugin_root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy-github")
}

pub(super) fn repository(root: &Path, name: &str, remote: &str) -> TestResult<PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    std::fs::write(
        path.join(".git/config"),
        format!("[remote \"origin\"]\n\turl = {remote}\n"),
    )?;
    Ok(path)
}
