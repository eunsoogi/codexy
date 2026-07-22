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
    let rewritten = repository(
        workspace.path(),
        "rewritten",
        &["shortcut:codexy.git"],
    )?;
    std::fs::write(
        rewritten.join(".git/config"),
        "[remote \"origin\"]\n\turl = shortcut:codexy.git\n[url \"git@github.com:eunsoogi/\"]\n\tinsteadOf = shortcut:\n",
    )?;
    for command in [
        "GIT_DIR=owned/.git git -C foreign push --force origin topic",
        "GIT_DIR=owned/.git env -C foreign git push --force origin topic",
        "bash -lc 'git push --force git@github.com:eunsoogi/codexy.git topic'",
        "eval 'git push --force' 'git@github.com:eunsoogi/codexy.git topic'",
        "GH_REPO=github.com/eunsoogi/codexy gh pr create --title 'Plain title'",
        "eval -- 'git push --force' 'git@github.com:eunsoogi/codexy.git topic'",
        "env -Cowned git push --force origin topic",
        "cd -L owned && git push --force origin topic",
        "cd -P owned && git push --force origin topic",
        "sudo -D owned git push --force origin topic",
        "sudo --chdir=owned git push --force origin topic",
        "exec -a policy git push --force origin topic",
        "exec -c git push --force origin topic",
        "exec -l git push --force origin topic",
        "nohup git push --force origin topic",
        "git -c alias.push=!git\\ push\\ --force\\ git@github.com:eunsoogi/codexy.git\\ topic push",
        "if true; then cd owned; git push --force origin topic; fi",
    ] {
        let output = bash(&root, workspace.path(), command)?;
        assert_deny(&output).map_err(|error| format!("{command}: {output:?}: {error}"))?;
    }
    assert_eq!(bash(&root, &foreign, "GIT_DIR=owned/.git env -uGIT_DIR git push --force origin topic")?, b"");
    assert_eq!(bash(&root, &foreign, "GH_REPO=eunsoogi/codexy env -uGH_REPO gh pr create --title 'Plain title'")?, b"");
    assert_eq!(bash(&root, &foreign, "eval 'printf %s' git@github.com:eunsoogi/codexy.git")?, b"");
    assert_eq!(bash(&root, &foreign, "eval -- 'printf %s' git@github.com:eunsoogi/codexy.git")?, b"");
    assert_eq!(bash(&root, &multi_foreign, "git reset --hard HEAD")?, b"");
    let rewritten_output = bash(&root, &rewritten, "git push --force origin topic")?;
    assert_deny(&rewritten_output).map_err(|error| format!("rewrite: {rewritten_output:?}: {error}"))?;
    let missing_body = github(&root, "Missing body")?;
    assert_deny(&missing_body).map_err(|error| format!("body: {missing_body:?}: {error}"))?;
    let hidden_body = github(&root, "```markdown\n## Problem\n```\n## Scope\n## Acceptance Criteria\n## Verification")?;
    assert_deny(&hidden_body).map_err(|error| format!("hidden body: {hidden_body:?}: {error}"))?;
    assert_eq!(github(&root, "## Problem\nA\n## Scope\nB\n## Acceptance Criteria\nC\n## Verification\nD")?, b"");
    assert!(owned.exists());
    Ok(())
}

#[test]
fn command_position_expansion_is_classified() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", &["git@github.com:eunsoogi/codexy.git"])?;
    assert_deny(&bash(&root, &owned, "X='git push --force origin topic'; $X")?)?;
    assert_eq!(bash(&root, &owned, "X='printf safe'; $X")?, b"");
    Ok(())
}

#[test]
fn typed_graphql_query_file_is_classified() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", &["git@github.com:eunsoogi/codexy.git"])?;
    let mutation = workspace.path().join("merge.graphql");
    std::fs::write(&mutation, "mutation { mergePullRequest(input: { pullRequestId: \"PR\", mergeMethod: MERGE }) { pullRequest { id } } }")?;
    let viewer = workspace.path().join("viewer.graphql");
    std::fs::write(&viewer, "query { viewer { login } }")?;
    assert_deny(&bash(&root, &owned, &format!("gh api graphql -F query=@{}", mutation.display()))?)?;
    assert_eq!(bash(&root, &owned, &format!("gh api graphql -F query=@{}", viewer.display()))?, b"");
    Ok(())
}

fn github(root: &std::path::Path, body: &str) -> TestResult<Vec<u8>> {
    let input = json!({"hook_event_name":"PreToolUse","tool_name":"mcp__codex_apps__github_create_issue","tool_input":{"repository_full_name":"eunsoogi/codexy","title":"Improve hooks","body":body}});
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg("PreToolUse").env_clear().env("PLUGIN_ROOT", root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(&input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
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
