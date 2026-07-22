use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn official_event_outputs_are_deterministic_deny_or_zero() -> TestResult {
    let root = root();
    let other = json!({"hook_event_name":"PreToolUse","tool_name":"mcp__filesystem__read_file","tool_input":{"path":"/tmp/a"}});
    assert_eq!(run(&root, "PreToolUse", &other)?, b"");
    let malformed = run_bytes(&root, "PreToolUse", b"{not-json")?;
    assert_eq!(malformed, run_bytes(&root, "PreToolUse", b"{not-json")?);
    assert_deny(&malformed, "PreToolUse")?;
    let permission = run_bytes(&root, "PermissionRequest", b"{not-json")?;
    let value: Value = serde_json::from_slice(&permission)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
    assert_eq!(value["hookSpecificOutput"]["decision"]["behavior"], "deny");
    assert!(value["hookSpecificOutput"].get("permissionDecision").is_none());
    Ok(())
}

#[test]
fn thread_routing_requires_explicit_model_and_thinking_only() -> TestResult {
    let root = root();
    for input in [json!({}), json!({"model":"gpt-5.6-sol"}), json!({"thinking":"high"}),
        json!({"model":null,"thinking":"high"}), json!({"model":"gpt-5.6-sol","thinking":"  "})]
    {
        assert_deny(&run(&root, "PreToolUse", &payload("codex_app__send_message_to_thread", input, None))?, "PreToolUse")?;
    }
    assert_eq!(run(&root, "PreToolUse", &payload("codex_app__send_message_to_thread",
        json!({"threadId":"x","prompt":"canary","model":"explicit","thinking":"explicit"}), None))?, b"");
    Ok(())
}

#[test]
fn github_owned_mutations_follow_canonical_contracts() -> TestResult {
    let root = root();
    for title in ["Fix: broken", " Improve hooks", "Ämprove hooks"] {
        assert_deny(&github(&root, "github_create_issue", json!({"title":title}))?, "PreToolUse")?;
    }
    assert_eq!(github(&root, "github_create_issue", json!({"title":"Improve hooks"}))?, b"");
    for title in ["fix(a.b): invalid scope", " fix(hooks): leading"] {
        assert_deny(&github(&root, "github_create_pull_request", json!({"title":title}))?, "PreToolUse")?;
    }
    assert_eq!(github(&root, "github_create_pull_request", json!({"title":"security: harden"}))?, b"");
    assert_deny(&github(&root, "github_update_issue", json!({"issue_number":true}))?, "PreToolUse")?;
    assert_deny(&github(&root, "github_enable_auto_merge", json!({"pr_number":453}))?, "PreToolUse")?;
    assert_deny(&github(&root, "github_create_issue", json!({"title":"bad","unknown":1}))?, "PreToolUse")?;
    assert_deny(&github_repo(&root, "Eunsoogi/Codexy", "github_create_issue", json!({"title":"bad"}))?, "PreToolUse")?;
    assert_eq!(github_repo(&root, "openai/codex", "github_create_issue", json!({"title":"bad"}))?, b"");
    Ok(())
}

#[test]
fn squash_merge_requires_head_subject_and_one_final_closing_reference() -> TestResult {
    let root = root();
    let valid = json!({"pr_number":479,"merge_method":"squash","expected_head_sha":"a".repeat(40),
        "commit_title":"fix(hooks): enforce policy (#479)","commit_message":"Summary\n\nFixes #453"});
    assert_eq!(github(&root, "github_merge_pull_request", valid.clone())?, b"");
    for invalid in [
        json!({"pr_number":true,"merge_method":"squash","expected_head_sha":"a".repeat(40),"commit_title":"fix: x (#453)","commit_message":"Fixes #453"}),
        json!({"pr_number":453,"merge_method":"squash","expected_head_sha":"a".repeat(40),"commit_title":"fix: x (#453)","commit_message":"Closes #1\nFixes #453"}),
        json!({"pr_number":453,"merge_method":"merge","expected_head_sha":"a".repeat(40),"commit_title":"fix: x (#453)","commit_message":"Fixes #453"}),
    ] { assert_deny(&github(&root, "github_merge_pull_request", invalid)?, "PreToolUse")?; }
    Ok(())
}

#[test]
fn shell_policy_blocks_structural_bypasses_without_substring_false_positives() -> TestResult {
    let root = root();
    let owned = repository("git@github.com:eunsoogi/codexy.git")?;
    let other = repository("https://github.com/openai/codex.git")?;
    for command in ["echo ok\ngit push --force origin main", "echo $(git push --force origin main)",
        "exec git push --force origin main", "FOO=1 git push --force origin main",
        "git -c alias.x='push --force' x origin main", "git push origin +main",
        "git push --mirror origin", "sudo -u root git push --force origin topic",
        "printf '%s\\n' origin topic | xargs git push --force",
        "gh pr create --title 'Plain title' --body x",
        "gh pr merge 453 --admin=true", "rm -rf ${HOME}", "echo x | sh"]
    {
        assert_deny(&bash(&root, owned.path(), command)?, "PreToolUse")?;
    }
    assert_eq!(bash(&root, owned.path(), "printf '%s' 'git push --force is documented'")?, b"");
    assert_eq!(bash(&root, other.path(), "git push --force origin topic")?, b"");
    assert_deny(&bash(&root, other.path(), "git push --force git@github.com:eunsoogi/codexy.git main")?, "PreToolUse")?;
    assert_deny(&bash(&root, other.path(), "echo $(git push --force git@github.com:eunsoogi/codexy.git topic)")?, "PreToolUse")?;
    let changed_directory = format!("git -C {} push --force origin main", owned.path().display());
    assert_deny(&bash(&root, other.path(), &changed_directory)?, "PreToolUse")?;
    assert_eq!(bash(&root, owned.path(), "gh pr create --title 'fix(hooks): valid title' --body x")?, b"");
    assert_eq!(bash(&root, other.path(), "echo $(printf safe)")?, b"");
    Ok(())
}

#[test]
fn packaged_launcher_is_plugin_local_and_does_not_write_state() -> TestResult {
    let source = root();
    let extracted = tempfile::tempdir()?;
    copy_tree(&source, extracted.path())?;
    let state = tempfile::tempdir()?;
    let before = snapshot(state.path())?;
    let output = run_with_state(extracted.path(), state.path(), "PreToolUse",
        &payload("mcp__filesystem__read_file", json!({"path":"/tmp/a"}), None))?;
    assert_eq!(output, b"");
    assert_eq!(snapshot(state.path())?, before);
    assert!(!contains_pycache(extracted.path())?);
    Ok(())
}

fn github(root: &std::path::Path, suffix: &str, fields: Value) -> TestResult<Vec<u8>> {
    github_repo(root, "eunsoogi/codexy", suffix, fields)
}

fn github_repo(root: &std::path::Path, repository: &str, suffix: &str, fields: Value) -> TestResult<Vec<u8>> {
    let mut input = fields.as_object().cloned().ok_or("fields")?;
    input.insert("repository_full_name".into(), json!(repository));
    run(root, "PreToolUse", &payload(&format!("mcp__codex_apps__{suffix}"), Value::Object(input), None))
}

fn bash(root: &std::path::Path, cwd: &std::path::Path, command: &str) -> TestResult<Vec<u8>> {
    run(root, "PreToolUse", &payload("Bash", json!({"command":command}), Some(cwd)))
}

fn payload(tool: &str, input: Value, cwd: Option<&std::path::Path>) -> Value {
    let mut value = json!({"hook_event_name":"PreToolUse","tool_name":tool,"tool_input":input});
    if let Some(cwd) = cwd { value["cwd"] = json!(cwd); }
    value
}

fn run(root: &std::path::Path, event: &str, value: &Value) -> TestResult<Vec<u8>> {
    run_bytes(root, event, &serde_json::to_vec(value)?)
}

fn run_bytes(root: &std::path::Path, event: &str, input: &[u8]) -> TestResult<Vec<u8>> {
    raw(root, event, input, None)
}

fn run_with_state(root: &std::path::Path, state: &std::path::Path, event: &str, value: &Value) -> TestResult<Vec<u8>> {
    raw(root, event, &serde_json::to_vec(value)?, Some(state))
}

fn raw(root: &std::path::Path, event: &str, input: &[u8], state: Option<&std::path::Path>) -> TestResult<Vec<u8>> {
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg(event).env_clear().env("PLUGIN_ROOT", root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(state) = state { child.env("HOME", state).env("CODEX_HOME", state).env("PLUGIN_DATA", state).env("XDG_CACHE_HOME", state); }
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(input)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
}

fn assert_deny(bytes: &[u8], event: &str) -> TestResult {
    let value: Value = serde_json::from_slice(bytes)?;
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], event);
    Ok(())
}

fn repository(remote: &str) -> TestResult<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    std::fs::create_dir(temp.path().join(".git"))?;
    std::fs::write(temp.path().join(".git/config"), format!("[remote \"origin\"]\n\turl = {remote}\n"))?;
    Ok(temp)
}

fn root() -> std::path::PathBuf { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy") }
fn snapshot(path: &std::path::Path) -> TestResult<Vec<String>> { Ok(std::fs::read_dir(path)?.map(|e| e.map(|v| v.file_name().to_string_lossy().into_owned())).collect::<Result<_,_>>()?) }
fn contains_pycache(path: &std::path::Path) -> TestResult<bool> { for e in walk(path)? { if e.file_name().is_some_and(|n| n == "__pycache__") { return Ok(true); } } Ok(false) }
fn walk(path: &std::path::Path) -> TestResult<Vec<std::path::PathBuf>> { let mut out=Vec::new(); for e in std::fs::read_dir(path)? { let p=e?.path(); out.push(p.clone()); if p.is_dir(){out.extend(walk(&p)?);} } Ok(out) }
fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> { std::fs::create_dir_all(target)?; for e in std::fs::read_dir(source)? { let e=e?; let t=target.join(e.file_name()); if e.file_type()?.is_dir(){copy_tree(&e.path(),&t)?;}else{std::fs::copy(e.path(),t)?;} } Ok(()) }
