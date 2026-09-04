use std::{ffi::OsString, io::Write as _, path::Path, process::Stdio};

use crate::support::FixtureCommand as Command;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TOOL: &str = "spawn_agent";

fn launcher_command(launcher: &Path) -> Vec<OsString> {
    vec![
        "/d".into(),
        "/s".into(),
        "/c".into(),
        "call".into(),
        launcher.as_os_str().to_owned(),
        "PreToolUse".into(),
    ]
}

#[test]
fn launcher_command_keeps_path_unquoted_for_cmd() -> TestResult {
    let launcher = launcher_path();
    let args = launcher_command(&launcher);
    assert_eq!(args.len(), 6);
    assert_eq!(args[0], "/d");
    assert_eq!(args[1], "/s");
    assert_eq!(args[2], "/c");
    assert_eq!(args[3], "call");
    assert_eq!(args[4], launcher.as_os_str());
    assert!(!args[4].to_string_lossy().contains('"'));
    assert_eq!(args[5], "PreToolUse");
    Ok(())
}

#[test]
fn launcher_ignores_a_current_directory_py_cmd() -> TestResult {
    let temp = tempfile::tempdir()?;
    let sentinel = temp.path().join("shadowed.txt");
    std::fs::write(
        temp.path().join("py.cmd"),
        format!("@echo shadowed>\"{}\"\r\n@exit /b 0\r\n", sentinel.display()),
    )?;
    let mut child = Command::new("cmd.exe")
        .args(launcher_command(&launcher_path()))
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(&payload())?)?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "status={:?} stdout={} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!sentinel.exists(), "current-directory py.cmd executed");
    assert!(String::from_utf8_lossy(&output.stdout).contains("permissionDecision"));
    Ok(())
}

fn launcher_path() -> std::path::PathBuf {
    codexy_runtime::paths::repository_root()
        .join("plugins")
        .join("codexy")
        .join("hooks")
        .join("codexy-subagent-ownership.cmd")
}

fn payload() -> Value {
    json!({
        "hook_event_name": "PreToolUse",
        "tool_name": TOOL,
        "tool_input": {
            "task_name": "bounded_task",
            "agent_type": "worker",
            "message": "Implement issue #145."
        }
    })
}
