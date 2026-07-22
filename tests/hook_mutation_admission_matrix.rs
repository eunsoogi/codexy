use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PR_BODY: &str = "## Summary\nA\n## Rationale\nB\n## Changed Areas\nC\n## Verification\nD\n## Evidence\nE\n## Not Run\nNone\n## Follow-ups\nNone\n\nFixes #453";
const ISSUE_BODY: &str =
    "## Problem\nA\n## Scope\nB\n## Acceptance Criteria\nC\n## Verification\nD";

#[test]
fn env_split_string_preserves_trailing_arguments() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(
        &root,
        &foreign,
        "env -S 'git push' --force git@github.com:eunsoogi/codexy.git topic",
        true,
    )?;
    assert_case(
        &root,
        &foreign,
        "env --split-string='git push' --force git@github.com:eunsoogi/codexy.git topic",
        true,
    )?;
    assert_case(&root, &foreign, "env -S 'printf safe'", false)?;
    assert_case(&root, &foreign, "env -S 'git push' origin topic", false)
}

#[test]
fn inherited_git_dir_survives_shell_interpreters() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let command = format!(
        "GIT_DIR='{}/.git' bash -c 'git push --force origin topic'",
        owned.display()
    );
    assert_case(&root, &foreign, &command, true)?;
    let command = format!(
        "GIT_DIR='{}/.git' bash -c \"sh -c 'git push --force origin topic'\"",
        owned.display()
    );
    assert_case(&root, &foreign, &command, true)?;
    let command = format!(
        "GIT_DIR='{}/.git' env -u GIT_DIR bash -c 'git push --force origin topic'",
        owned.display()
    );
    assert_case(&root, &foreign, &command, false)
}

#[test]
fn effective_execution_context_preserves_supported_shell_state() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;

    for command in [
        "! git push --force origin topic",
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=alias.ship GIT_CONFIG_VALUE_0='!git push --force git@github.com:eunsoogi/codexy.git topic' git ship",
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=alias.ship GIT_CONFIG_VALUE_0='!git push --force git@github.com:eunsoogi/codexy.git topic' bash -c 'git ship'",
    ] {
        let cwd = if command.starts_with('!') { &owned } else { &foreign };
        assert_case(&root, cwd, command, true)?;
    }
    for command in [
        "REMOTE=git@github.com:eunsoogi/codexy.git; git push --force \"$REMOTE\" topic",
        "REPOSITORY=eunsoogi/codexy; GH_REPO=$REPOSITORY gh pr create --title 'Plain title'",
        "git push --force \"$UNKNOWN_REMOTE\" topic",
        "GH_REPO=$UNKNOWN_REPOSITORY gh pr create --title 'Plain title'",
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=alias.inspect git status",
        "GIT_CONFIG_COUNT=invalid git status",
        "GIT_CONFIG_KEY_0=alias.inspect GIT_CONFIG_VALUE_0=status git status",
        "GIT_CONFIG_COUNT=0 GIT_CONFIG_KEY_0=alias.inspect GIT_CONFIG_VALUE_0=status git status",
    ] {
        assert_case(&root, &foreign, command, true)?;
    }

    for command in [
        "! printf safe",
        "! git status",
        "GIT_CONFIG_COUNT=0 git status",
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=alias.inspect GIT_CONFIG_VALUE_0=status git inspect",
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=alias.inspect GIT_CONFIG_VALUE_0=status bash -c 'git inspect'",
        "REMOTE=git@github.com:eunsoogi/codexy.git; printf '%s' \"$REMOTE\"",
        "REMOTE=git@github.com:eunsoogi/codexy.git; git push \"$REMOTE\" topic",
    ] {
        assert_case(&root, &foreign, command, false)?;
    }
    assert_case(
        &root,
        &foreign,
        &format!(
            "GIT_DIR='{}/.git' env -i git push --force origin topic",
            owned.display()
        ),
        false,
    )?;
    Ok(())
}

#[test]
fn merge_admission_requires_canonical_squash() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    assert_case(&root, &owned, "gh pr merge 453 --merge", true)?;
    assert_case(&root, &owned, "gh pr merge --rebase 453", true)?;
    assert_case(&root, &owned, "gh pr merge 453 --squash", false)?;
    assert_case(
        &root,
        &owned,
        "gh pr merge --delete-branch --squash --match-head-commit aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa 453",
        false,
    )
}

#[test]
fn issue_create_requires_valid_body_evidence() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let issue_body = owned.join("issue.md");
    let malformed = owned.join("malformed.md");
    std::fs::write(&issue_body, ISSUE_BODY)?;
    std::fs::write(&malformed, "arbitrary")?;
    assert_case(&root, &owned, "gh issue create --title 'Improve hooks' --body arbitrary", true)?;
    assert_case(&root, &owned, "gh issue create --title 'Improve hooks'", true)?;
    assert_case(
        &root,
        &owned,
        &format!("gh issue create --title 'Improve hooks' --body-file '{}'", malformed.display()),
        true,
    )?;
    assert_case(
        &root,
        &owned,
        &format!("gh issue create --body-file '{}' --title 'Improve hooks'", issue_body.display()),
        false,
    )?;
    assert_case(
        &root,
        &owned,
        &format!("gh issue create --title 'Improve hooks' --body '{}'", ISSUE_BODY),
        false,
    )
}

#[test]
fn pr_create_accepts_one_regular_body_file() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let pr_body = owned.join("pr.md");
    std::fs::write(&pr_body, PR_BODY)?;
    assert_case(
        &root,
        &owned,
        &format!("gh pr create --head topic --body-file '{}' --base main --title 'fix(hooks): enforce admission'", pr_body.display()),
        false,
    )?;
    assert_case(
        &root,
        &owned,
        &format!("gh pr create --title 'fix(hooks): enforce admission' --body-file '{}' --body '{}'", pr_body.display(), PR_BODY),
        true,
    )?;
    assert_case(&root, &owned, "gh pr create --title 'fix(hooks): enforce admission' --body-file missing.md", true)
}

#[test]
fn connector_and_cli_adapters_share_body_contract() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    assert_connector(&root, "github_create_issue", json!({"title":"Improve hooks","body":"arbitrary"}), true)?;
    assert_connector(&root, "github_create_issue", json!({"title":"Improve hooks","body":ISSUE_BODY}), false)?;
    assert_connector(&root, "github_create_pull_request", json!({"title":"fix(hooks): enforce admission","body":PR_BODY}), false)?;
    assert_connector(&root, "github_update_issue", json!({"issue_number":453,"labels":["type/fix"]}), false)?;
    assert_connector(&root, "github_update_pull_request", json!({"pr_number":479,"state":"open"}), false)?;
    assert_case(&root, &owned, "gh issue edit 453 --add-label type/fix", false)?;
    assert_case(&root, &owned, "gh pr edit 479 --add-label type/fix", false)
}

fn assert_case(root: &std::path::Path, cwd: &std::path::Path, command: &str, denied: bool) -> TestResult {
    assert_decision(&bash(root, cwd, command)?, denied, command)
}

fn assert_connector(root: &std::path::Path, operation: &str, input: Value, denied: bool) -> TestResult {
    let tool_name = format!("mcp__codex_apps__{operation}");
    let mut fields = input.as_object().cloned().ok_or("connector object")?;
    fields.insert("repository_full_name".to_owned(), json!("eunsoogi/codexy"));
    let payload = json!({"hook_event_name":"PreToolUse","tool_name":tool_name,"tool_input":fields});
    assert_decision(&hook(root, &payload)?, denied, operation)
}

fn assert_decision(output: &[u8], denied: bool, label: &str) -> TestResult {
    if denied {
        let value: Value = serde_json::from_slice(output)
            .map_err(|error| format!("expected deny for {label:?}, got {output:?}: {error}"))?;
        assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny", "{label}");
    } else {
        assert_eq!(output, b"", "{label}");
    }
    Ok(())
}

fn bash(root: &std::path::Path, cwd: &std::path::Path, command: &str) -> TestResult<Vec<u8>> {
    let input = json!({"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":command},"cwd":cwd});
    hook(root, &input)
}

fn hook(root: &std::path::Path, input: &Value) -> TestResult<Vec<u8>> {
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg("PreToolUse").env_clear().env("PLUGIN_ROOT", root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
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
