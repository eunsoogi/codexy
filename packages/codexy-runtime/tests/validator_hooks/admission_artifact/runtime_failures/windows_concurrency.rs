use super::{LAUNCHERS, Stdio, copy};
use std::io::Write as _;
use std::sync::{Arc, Barrier};

#[test]
fn native_windows_launchers_keep_concurrent_output_isolated_and_clean() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    for launcher in LAUNCHERS {
        std::fs::write(
            root.join(format!("hooks/{launcher}.py")),
            "import json, sys, time\ntime.sleep(0.2)\nevent = sys.argv[2]\nreason = 'CODEXY_CONCURRENT_OUTPUT'\nbody = {'hookSpecificOutput': {'hookEventName': event, 'decision': {'behavior': 'deny', 'message': reason}}} if event == 'PermissionRequest' else {'hookSpecificOutput': {'hookEventName': event, 'permissionDecision': 'deny', 'permissionDecisionReason': reason}}\nprint(json.dumps(body))\n",
        )?;
    }
    let starts = Arc::new(Barrier::new(LAUNCHERS.len() * 2));
    let mut joins = Vec::new();
    for event in ["PermissionRequest", "PreToolUse"] {
        for launcher in LAUNCHERS {
            let root = root.clone();
            let cwd = temp.path().to_path_buf();
            let starts = Arc::clone(&starts);
            joins.push(std::thread::spawn(move || -> Result<_, std::io::Error> {
                starts.wait();
                let input = serde_json::json!({
                    "hook_event_name": event,
                    "tool_name": "Bash",
                    "tool_input": null,
                    "cwd": cwd,
                });
                let mut child = std::process::Command::new("cmd");
                child.arg("/d").arg("/c")
                    .arg(root.join(format!("hooks/{launcher}.cmd"))).arg(event)
                    .env("PLUGIN_ROOT", &root).stdin(Stdio::piped())
                    .stdout(Stdio::piped()).stderr(Stdio::piped());
                let mut child = child.spawn()?;
                child.stdin.take().expect("launcher stdin")
                    .write_all(&serde_json::to_vec(&input).expect("input json"))?;
                Ok((launcher, event, child.wait_with_output()?))
            }));
        }
    }
    for join in joins {
        let (launcher, event, output) = join.join().expect("concurrent launcher thread")?;
        assert!(output.status.success(), "{event} {launcher}");
        assert!(output.stderr.is_empty(), "{event} {launcher}");
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        if event == "PermissionRequest" {
            assert_eq!(denial["hookSpecificOutput"]["decision"]["behavior"], "deny");
        } else {
            assert_eq!(denial["hookSpecificOutput"]["permissionDecision"], "deny");
        }
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_CONCURRENT_OUTPUT"));
    }
    let leftovers = std::fs::read_dir(root.join("hooks"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("codexy-hook-"))
        .count();
    assert_eq!(leftovers, 0, "temporary launcher output leaked");
    Ok(())
}
