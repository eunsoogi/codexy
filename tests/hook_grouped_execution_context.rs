use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn grouped_commands_inspect_nested_mutations_and_context() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let owned_path = owned.display();
    for command in [
        format!("(cd '{owned_path}' && git push --force origin topic)"),
        format!("{{ cd '{owned_path}' && git push --force origin topic; }}"),
        format!("({{ cd '{owned_path}'; git push --force origin topic; }})"),
        "(git push --force git@github.com:eunsoogi/codexy.git topic)".into(),
        format!("(GIT_DIR='{owned_path}/.git'; git push --force origin topic)"),
        "(GH_REPO=eunsoogi/codexy; gh pr merge 453 --merge)".into(),
        "(printf $(git push --force git@github.com:eunsoogi/codexy.git topic))".into(),
    ] {
        assert_case(&root, &foreign, &command, true)?;
    }
    assert_case(
        &root,
        &owned,
        "(cd ../foreign && git push --force git@github.com:eunsoogi/codexy.git topic)",
        true,
    )?;
    for command in [
        "(printf safe; git push --force origin topic)",
        "(printf safe && git push --force origin topic)",
        "(false || git push --force origin topic)",
        "(printf safe | git push --force origin topic)",
        "(printf safe & git push --force origin topic)",
    ] {
        assert_case(&root, &owned, command, true)?;
    }
    Ok(())
}

#[test]
fn grouped_commands_preserve_benign_and_foreign_controls() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for (cwd, command) in [
        (&foreign, "(printf safe)"),
        (&foreign, "{ printf safe; }"),
        (&foreign, "((printf safe))"),
        (&foreign, "{ printf safe && git status; }"),
        (&foreign, "{ false || printf safe; }"),
        (&foreign, "(printf safe | tr a-z A-Z)"),
        (&foreign, "(printf $(date))"),
        (&foreign, "! (printf safe)"),
        (&foreign, "(cd ../owned & git push --force origin topic)"),
        (&foreign, "(git push --force https://github.com/openai/codex.git topic)"),
        (&owned, "(git status)"),
        (&owned, "{ git status; }"),
        (&owned, "(cd ../foreign && git push --force origin topic)"),
    ] {
        assert_case(&root, cwd, command, false)?;
    }
    Ok(())
}

#[test]
fn subshell_isolates_context_while_braces_propagate_it() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, "(cd ../owned); git push --force origin topic", false)?;
    assert_case(&root, &foreign, "{ cd ../owned; }; git push --force origin topic", true)?;
    assert_case(&root, &owned, "(cd ../foreign); git push --force origin topic", true)?;
    assert_case(&root, &owned, "{ cd ../foreign; }; git push --force origin topic", false)
}

#[test]
fn malformed_or_opaque_groups_fail_closed() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for command in [
        "(printf safe",
        "{ printf safe;",
        "{ printf safe }",
        "printf safe)",
        "{ }",
        "(printf $((date))",
    ] {
        assert_case(&root, &foreign, command, true)?;
    }
    assert_case(&root, &owned, "(printf `date`)", true)
}

#[test]
fn cd_option_grammar_propagates_only_effective_directory_changes() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned space", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let owned_path = owned.display();
    for command in [
        format!("cd -P -e '{owned_path}' && git push --force origin topic"),
        format!("cd -Pe '{owned_path}' && git push --force origin topic"),
        format!("cd -eP -- '{owned_path}' && git push --force origin topic"),
        format!("cd -L -P -e '{owned_path}' && git push --force origin topic"),
        format!("{{ cd -P -e '{owned_path}'; git push --force origin topic; }}"),
        format!("(cd -P -e '{owned_path}' && git push --force origin topic)"),
        format!("false || cd -P -e '{owned_path}' && git push --force origin topic"),
        format!("cd -P -@ '{owned_path}' && git push --force origin topic"),
    ] {
        assert_case(&root, &foreign, &command, true)?;
    }
    for command in [
        format!("cd -e '{owned_path}' && git push --force origin topic"),
        format!("cd -P -e -L '{owned_path}' && git push --force origin topic"),
        format!("cd '{owned_path}' extra && git push --force origin topic"),
        format!("cd -Z '{owned_path}' && git push --force origin topic"),
    ] {
        assert_case(&root, &foreign, &command, false)?;
    }
    for command in ["cd", "cd --", "cd -", "cd -P -e"] {
        assert_case(&root, &foreign, command, true)?;
    }
    assert_case(
        &root,
        &owned,
        "(cd -P ../foreign); git push --force origin topic",
        true,
    )?;
    assert_case(
        &root,
        &owned,
        "{ cd -P ../foreign; }; git push --force origin topic",
        false,
    )
}

#[test]
fn bash_comments_do_not_change_cd_state() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;

    assert_case(
        &root,
        &foreign,
        &format!("cd '{}' # owned directory\ngit push --force origin topic", owned.display()),
        true,
    )?;
    assert_case(
        &root,
        &foreign,
        &format!("cd '{}'\ngit push --force origin topic", owned.display()),
        true,
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
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(&input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    if denied {
        assert!(!output.stdout.is_empty(), "expected denial: {command}");
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
    std::fs::write(path.join(".git/config"), format!("[remote \"origin\"]\n\turl = {remote}\n"))?;
    Ok(path)
}

fn plugin_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
}
