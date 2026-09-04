use super::{Command, Stdio, copy};
use super::LAUNCHERS;
use crate::support::fixture_native_launcher;
#[cfg(windows)]
use std::io::Write as _;

#[cfg(windows)]
#[path = "runtime_failures/windows_concurrency.rs"]
mod windows_concurrency;

#[test]
fn real_launchers_fail_closed_when_a_policy_module_is_missing()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::remove_file(root.join("hooks/codexy_policy/repository_issue.py"))?;
    assert_runtime_denial(&root, None)
}

#[cfg(unix)]
#[test]
fn runtime_ignores_hostile_path_interpreter_and_env_decoys()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let bin = temp.path().join("runtime-bin");
    std::fs::create_dir(&bin)?;
    let marker = temp.path().join("hostile-path-ran");
    for executable in ["env", "python3"] {
        let decoy = bin.join(executable);
        std::fs::write(&decoy, format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()))?;
        std::fs::set_permissions(&decoy, std::fs::Permissions::from_mode(0o755))?;
    }
    let output = Command::new(root.join("hooks/codexy-repository-issue.sh"))
        .arg("PreToolUse")
        .env("PLUGIN_ROOT", &root)
        .env("PATH", &bin)
        .stdin(Stdio::null())
        .output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(!marker.exists(), "PATH decoy ran");
    Ok(())
}

fn assert_runtime_denial(
    root: &std::path::Path,
    path: Option<&std::ffi::OsStr>,
) -> Result<(), Box<dyn std::error::Error>> {
    for event in ["PermissionRequest", "PreToolUse"] {
        let launcher = fixture_native_launcher(
            cfg!(windows),
            &root.join("hooks/codexy-repository-issue.sh"),
        )
        .ok_or("native repository issue launcher")?;
        let mut command = Command::new(launcher);
        command.arg(event).env("PLUGIN_ROOT", root).stdin(Stdio::null());
        if let Some(path) = path {
            command.env("PATH", path);
        }
        let output = command.output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty(), "runtime stderr leaked");
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_REPOSITORY_ISSUE_RUNTIME"));
    }
    Ok(())
}

#[test]
fn cmd_launchers_fail_closed_without_a_path_selected_interpreter()
-> Result<(), Box<dyn std::error::Error>> {
    let hooks = codexy_runtime::paths::repository_root().join("plugins/codexy-github/hooks");
    for launcher in LAUNCHERS {
        let source = std::fs::read_to_string(hooks.join(format!("{launcher}.cmd")))?;
        assert!(source.contains("DisableDelayedExpansion"), "{launcher}");
        if *launcher == "codexy-repository-issue"
            || *launcher == "codexy-repository-pull-request"
            || *launcher == "codexy-repository-github-command"
            || *launcher == "codexy-destructive-command"
        {
            assert!(source.contains("py -3 -I -B"), "{launcher}");
        } else {
            assert!(!source.contains("py "), "{launcher}");
        }
        assert!(!source.contains("powershell"), "{launcher}");
        assert!(!source.contains("%*"), "{launcher}");
    }
    Ok(())
}

#[test]
fn cmd_pull_request_launcher_invokes_packaged_policy_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let hooks = codexy_runtime::paths::repository_root().join("plugins/codexy-github/hooks");
    let source = std::fs::read_to_string(hooks.join("codexy-repository-pull-request.cmd"))?;
    assert!(source.contains("if /I \"%event%\"==\"PreToolUse\" goto evaluate"));
    assert!(source.contains("if /I \"%event%\"==\"PermissionRequest\" goto evaluate"));
    assert!(source.contains("py -3 -I -B -c \"import subprocess,sys;"));
    assert!(source.contains("codexy-repository-pull-request.py\" --event \"%event%\""));
    assert!(source.contains("sys.stdout.buffer.write(p.stdout if p.returncode==0 else b'')"));
    Ok(())
}

#[test]
fn cmd_destructive_launcher_invokes_packaged_policy_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let hooks = codexy_runtime::paths::repository_root().join("plugins/codexy-github/hooks");
    let source = std::fs::read_to_string(hooks.join("codexy-destructive-command.cmd"))?;
    assert!(source.contains("if /I \"%event%\"==\"PreToolUse\" goto evaluate"));
    assert!(source.contains("if /I \"%event%\"==\"PermissionRequest\" goto evaluate"));
    assert!(source.contains("py -3 -I -B -c \"import subprocess,sys;"));
    assert!(source.contains("codexy-destructive-command.py\" --event \"%event%\""));
    assert!(source.contains("sys.stdout.buffer.write(p.stdout if p.returncode==0 else b'')"));
    Ok(())
}

#[test]
fn cmd_launchers_have_no_temporary_interpreter_output()
-> Result<(), Box<dyn std::error::Error>> {
    let hooks = codexy_runtime::paths::repository_root().join("plugins/codexy-github/hooks");
    for launcher in LAUNCHERS {
        let source = std::fs::read_to_string(hooks.join(format!("{launcher}.cmd")))?;
        assert!(!source.contains("tempfile"), "{launcher}");
        assert!(!source.contains("%RANDOM%"), "{launcher}");
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn native_windows_launchers_execute_the_packaged_cmd_entrypoints()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    for event in ["PermissionRequest", "PreToolUse"] {
        for launcher in LAUNCHERS {
            let input = serde_json::json!({
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": null,
                "cwd": temp.path(),
            });
            let mut child = std::process::Command::new("cmd");
            child.arg("/d").arg("/c")
                .arg(root.join(format!("hooks/{launcher}.cmd"))).arg(event)
                .env("PLUGIN_ROOT", &root).stdin(Stdio::piped())
                .stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = child.spawn()?;
            child.stdin.take().ok_or("launcher stdin")?
                .write_all(&serde_json::to_vec(&input)?)?;
            let output = child.wait_with_output()?;
            assert!(output.status.success(), "{event} {launcher}");
            assert!(output.stderr.is_empty(), "{event} {launcher}");
            let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        }
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn native_windows_launchers_discard_partial_child_stdout_before_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    for launcher in LAUNCHERS {
        std::fs::write(
            root.join(format!("hooks/{launcher}.py")),
            "import sys\nsys.stdout.write('{\\\"partial\\\":')\nsys.exit(1)\n",
        )?;
        for event in ["PermissionRequest", "PreToolUse"] {
            let input = serde_json::json!({
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": null,
                "cwd": temp.path(),
            });
            let mut child = std::process::Command::new("cmd");
            child.arg("/d").arg("/c")
                .arg(root.join(format!("hooks/{launcher}.cmd"))).arg(event)
                .env("PLUGIN_ROOT", &root).stdin(Stdio::piped())
                .stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = child.spawn()?;
            child.stdin.take().ok_or("launcher stdin")?
                .write_all(&serde_json::to_vec(&input)?)?;
            let output = child.wait_with_output()?;
            assert!(output.status.success(), "{event} {launcher}");
            assert!(output.stderr.is_empty(), "{event} {launcher}");
            let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
            assert!(
                String::from_utf8(output.stdout)?.contains(diagnostic(launcher)),
                "{event} {launcher}"
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
fn diagnostic(launcher: &str) -> &'static str {
    match launcher {
        "codexy-repository-issue" => "CODEXY_REPOSITORY_ISSUE_RUNTIME",
        "codexy-repository-pull-request" => "CODEXY_REPOSITORY_PULL_REQUEST_RUNTIME",
        "codexy-repository-merge" => "CODEXY_REPOSITORY_MERGE_RUNTIME",
        "codexy-repository-github-command" => "CODEXY_REPOSITORY_GITHUB_COMMAND_RUNTIME",
        "codexy-destructive-command" => "CODEXY_DESTRUCTIVE_COMMAND_RUNTIME",
        _ => unreachable!("known launcher"),
    }
}
