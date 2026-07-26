use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn opaque_graphql_mutations_fail_closed_without_blocking_queries() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, "gh api graphql -f query='mutation { mergePullRequest(input:{pullRequestId:\"PR_owned_node\",mergeMethod:MERGE}) { pullRequest { id } } }'", true, &[])?;
    assert_case(&root, &foreign, "gh api graphql -f owner=openai -f name=codex -f query='mutation { mergePullRequest(input:{pullRequestId:\"PR_owned_node\",mergeMethod:MERGE}) { pullRequest { id } } }'", true, &[])
}

#[test]
fn graphql_comments_and_strings_remain_read_only_controls() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for command in [
        "gh api graphql -f query='query { viewer { login } }'",
        "gh api graphql -f query='query { viewer { login } }\n# mutation { mergePullRequest }'",
        "gh api graphql -f query='query { search(query:\"mutation { mergePullRequest }\",type:ISSUE,first:1) { issueCount } }'",
        "gh api graphql -f query='query mutation { viewer { login } }'",
        "gh api graphql -f query='{ mutation: viewer { login } }'",
        "gh api graphql -f query='query { ...mutation } fragment mutation on Query { viewer { login } }'",
    ] { assert_case(&root, &foreign, command, false, &[])?; }
    Ok(())
}

#[test]
fn malformed_graphql_escapes_fail_closed() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, r#"gh api graphql -f query='query { search(query:\"bad\q\",type:ISSUE,first:1) { issueCount } }'"#, true, &[])
}

#[test]
fn hash_path_aliases_cannot_disguise_git_mutations() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    assert_case(&root, &owned, "hash -p /usr/bin/git safe; safe push --force origin topic", true, &[])?;
    assert_case(&root, &owned, "hash -t git; git status --short", false, &[])
}

#[test]
fn inherited_git_common_dir_fails_closed_for_mutations_only() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let environment = [("GIT_DIR", foreign.join(".git")), ("GIT_COMMON_DIR", owned.join(".git"))];
    let environment = environment.iter().map(|(name, value)| (*name, value.as_os_str())).collect::<Vec<_>>();
    assert_case(&root, &foreign, "git push --force origin topic", true, &environment)?;
    assert_case(&root, &foreign, "git status --short", false, &environment)
}

#[test]
fn thread_delivery_requires_nonempty_model_and_thinking() -> TestResult {
    let root = plugin_root();
    assert_tool_case(
        &root,
        "codex_app__send_message_to_thread",
        json!({"threadId":"parent","model":"gpt-5.6-sol","thinking":"medium"}),
        false,
    )?;
    for input in [
        json!({"threadId":"parent","thinking":"medium"}),
        json!({"threadId":"parent","model":null,"thinking":"medium"}),
        json!({"threadId":"parent","model":"","thinking":"medium"}),
        json!({"threadId":"parent","model":"gpt-5.6-sol"}),
        json!({"threadId":"parent","model":"gpt-5.6-sol","thinking":null}),
        json!({"threadId":"parent","model":"gpt-5.6-sol","thinking":""}),
    ] {
        assert_tool_case(&root, "codex_app__send_message_to_thread", input, true)?;
    }
    Ok(())
}

#[test]
fn ordinary_launcher_variables_do_not_make_unrelated_commands_opaque() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, "printf '%s\\n' \"$HOME:$PATH:$USER\"", false, &[])?;
    assert_case(&root, &foreign, "printf '%s\\n' \"$UNKNOWN_RUNTIME_VALUE\"", true, &[])
}

#[cfg(unix)]
#[test]
fn filesystem_aliases_cannot_disguise_git_mutations() -> TestResult {
    use std::os::unix::fs::symlink;

    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let git = executable("git")?;
    let linked = owned.join("git-linked");
    let copied = owned.join("git-copied");
    symlink(&git, &linked)?;
    std::fs::copy(&git, &copied)?;
    assert_case(&root, &owned, "./git-linked push --force origin topic", true, &[])?;
    assert_case(&root, &owned, "./git-copied push --force origin topic", true, &[])?;
    assert_case(&root, &owned, "printf '%s\\n' benign", false, &[])
}

#[cfg(unix)]
#[test]
fn same_command_filesystem_aliases_cannot_disguise_git_mutations() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for command in [
        "ln -sf /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "cp /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "ln -sf /usr/bin/git safe && ./safe push --force origin topic",
        "cp /usr/bin/git safe && ./safe push --force origin topic",
        "cp -p /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "ln -sfn /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "ln -sf '/usr/bin/git' '/tmp/safe' && '/tmp/safe' push --force origin topic",
        "ln -sf \"$UNKNOWN_RUNTIME_VALUE\" /tmp/safe && /tmp/safe push --force origin topic",
        "ln -T /usr/bin/git safe && ./safe push --force origin topic",
        "cp -r /usr/bin/git safe && ./safe push --force origin topic",
    ] {
        assert_case(&root, &owned, command, true, &[])?;
    }
    for command in [
        "ln -sf /usr/bin/printf /tmp/safe && /tmp/safe '%s\\n' benign",
        "cp /usr/bin/printf /tmp/safe && /tmp/safe '%s\\n' benign",
        "ln -sf /usr/bin/printf safe && ./safe push --force origin topic",
        "ln -sf /usr/bin/git safe && ln -sf /usr/bin/printf safe && ./safe push --force origin topic",
    ] {
        assert_case(&root, &owned, command, false, &[])?;
    }
    Ok(())
}

pub(super) fn assert_case(root: &Path, cwd: &Path, command: &str, denied: bool, environment: &[(&str, &std::ffi::OsStr)]) -> TestResult {
    let input = json!({"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":command},"cwd":cwd});
    assert_input(root, input, denied, environment)
}

fn assert_tool_case(root: &Path, tool_name: &str, tool_input: Value, denied: bool) -> TestResult {
    assert_input(
        root,
        json!({"hook_event_name":"PreToolUse","tool_name":tool_name,"tool_input":tool_input}),
        denied,
        &[],
    )
}

fn assert_input(root: &Path, input: Value, denied: bool, environment: &[(&str, &std::ffi::OsStr)]) -> TestResult {
    let description = input.to_string();
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg("PreToolUse").env_clear().env("PLUGIN_ROOT", root).envs(environment.iter().copied()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(&input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    if denied { let value: Value = serde_json::from_slice(&output.stdout).map_err(|error| format!("expected deny for {description}: {error}"))?; assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny", "{description}"); } else { assert_eq!(output.stdout, b"", "{description}"); }
    Ok(())
}

#[cfg(unix)]
fn executable(name: &str) -> TestResult<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| format!("{name} executable"))
        .map_err(Into::into)
}

pub(super) fn repository(root: &Path, name: &str, remote: &str) -> TestResult<PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    std::fs::write(path.join(".git/config"), format!("[remote \"origin\"]\n\turl = {remote}\n"))?;
    Ok(path)
}

pub(super) fn plugin_root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy") }
