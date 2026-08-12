use super::{Command, Stdio, copy};
#[cfg(windows)]
use super::LAUNCHERS;
#[cfg(windows)]
use std::io::Write as _;

#[test]
fn real_launchers_fail_closed_when_a_policy_module_is_missing()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::remove_file(root.join("hooks/codexy_policy/thread_delivery.py"))?;
    assert_runtime_denial(&root, None)
}

#[cfg(unix)]
#[test]
fn runtime_probe_fails_closed_when_python_is_missing_or_incompatible()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;

    for python in [None, Some("#!/bin/sh\nexit 1\n")] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let bin = temp.path().join("runtime-bin");
        std::fs::create_dir(&bin)?;
        let env = bin.join("env");
        std::fs::write(
            &env,
            "#!/bin/sh\n[ \"${1-}\" = -i ] && shift\nwhile [ \"${1#*=}\" != \"$1\" ]; do shift; done\nexec \"$@\"\n",
        )?;
        std::fs::set_permissions(&env, std::fs::Permissions::from_mode(0o755))?;
        if let Some(source) = python {
            let executable = bin.join("python3");
            std::fs::write(&executable, source)?;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755))?;
        }
        assert_runtime_denial(&root, Some(bin.as_os_str()))?;
    }
    Ok(())
}

fn assert_runtime_denial(
    root: &std::path::Path,
    path: Option<&std::ffi::OsStr>,
) -> Result<(), Box<dyn std::error::Error>> {
    for event in ["PermissionRequest", "PreToolUse"] {
        let mut command = Command::new(root.join("hooks/codexy-thread-delivery.sh"));
        command.arg(event).env("PLUGIN_ROOT", root).stdin(Stdio::null());
        if let Some(path) = path {
            command.env("PATH", path);
        }
        let output = command.output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty(), "runtime stderr leaked");
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_THREAD_DELIVERY_RUNTIME"));
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
