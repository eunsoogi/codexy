use std::process::{Command, Output};
use std::time::{Duration, Instant};

#[allow(unused)]
mod support;

#[test]
fn validator_cli_bounds_session_start_context_execution() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    std::fs::write(&script_path, "#!/bin/sh\nsleep 30\n")?;

    let output = validate_hooks_with_deadline(&plugin_root, Duration::from_secs(6))?;
    assert!(
        !output.status.success(),
        "validator should reject a SessionStart hook that exceeds its timeout"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("timed out"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_bounds_output_collection_from_background_descendants()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    std::fs::write(
        &script_path,
        format!(
            "#!/bin/sh\n(trap '' TERM; sleep 30) &\nprintf '%s\\n' '{}'\n",
            session_start_context_json()
        ),
    )?;

    let output = validate_hooks_with_deadline(&plugin_root, Duration::from_secs(6))?;
    assert!(
        output.status.success(),
        "validator should accept valid hook output without waiting for inherited pipes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_kills_lingering_background_descendants_before_return()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let marker = format!("codexy-hook-descendant-{}", std::process::id());
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    std::fs::write(
        &script_path,
        format!(
            "#!/bin/sh\nsh -c 'trap \"\" TERM HUP; while :; do sleep 1; done' {} &\nprintf '%s\\n' '{}'\n",
            marker,
            session_start_context_json()
        ),
    )?;

    let output = validate_hooks_with_deadline(&plugin_root, Duration::from_secs(6))?;
    assert!(
        output.status.success(),
        "validator should accept valid hook output\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let pids = matching_pids(&marker)?;
    for pid in &pids {
        unsafe {
            let _ = libc::kill(*pid, libc::SIGKILL);
        }
    }
    assert!(
        pids.is_empty(),
        "validator returned while hook descendants were still running: {pids:?}"
    );
    Ok(())
}

#[test]
fn validator_cli_kills_redirected_background_descendants_before_return()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let marker = format!("codexy-hook-redirected-descendant-{}", std::process::id());
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    std::fs::write(
        &script_path,
        format!(
            "#!/bin/sh\nsh -c 'trap \"\" TERM HUP; : {}; exec 1<&- 2<&-; while :; do sleep 1; done' &\nprintf '%s\\n' '{}'\n",
            marker,
            session_start_context_json()
        ),
    )?;

    let output = validate_hooks_with_deadline(&plugin_root, Duration::from_secs(6))?;
    assert!(
        output.status.success(),
        "validator should accept valid hook output\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let pids = matching_pids(&marker)?;
    for pid in &pids {
        unsafe {
            let _ = libc::kill(*pid, libc::SIGKILL);
        }
    }
    assert!(
        pids.is_empty(),
        "validator returned while redirected hook descendants were still running: {pids:?}"
    );
    Ok(())
}

#[test]
fn validator_cli_bounds_continuous_hook_output() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    set_session_start_timeout(&plugin_root, 1)?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    std::fs::write(&script_path, "#!/bin/sh\nyes noisy-output\n")?;

    let output = validate_hooks_with_deadline(&plugin_root, Duration::from_secs(4))?;
    assert!(
        !output.status.success(),
        "validator should reject a SessionStart hook that writes continuously"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("output exceeded") || stderr.contains("timed out"),
        "unexpected stderr: {stderr}"
    );
    Ok(())
}

fn matching_pids(marker: &str) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let output = Command::new("pgrep").args(["-f", marker]).output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<i32>().ok())
        .filter(|pid| *pid != std::process::id() as i32)
        .collect())
}

fn session_start_context_json() -> &'static str {
    r#"{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"For compacted or resumed context hygiene, use $dreaming before continuing. Use Codexy codegraph MCP before direct file reads; include codegraph findings; record codegraph unavailable/uncallable fallback evidence; record registered-but-uncallable/unavailable-tool evidence. Use Codexy LSP; run lsp_status; record unavailable/not applicable evidence. Run hooks/codexy-issue-title-check.sh --issue-title before creating GitHub issues. Run hooks/codexy-pr-label-check.sh --pr-state-file pr-state.json with repositoryLabels before PR readiness. Run scripts/validate-plugin-config --check-completion-handoff for completion handoff claims. Before parent/orchestrator directives require hook entrypoints on a stacked child lane, validate those hook entrypoints against the child lane target base; if the target base lacks the hook path, name the available fallback validator command or record the mismatch as a separate dogfood defect instead of blocking the child on a future-branch path. Use hooks/codexy-pr-title-check.sh --pr-title and hooks/codexy-merge-message-check.sh --expected-pr PR_NUMBER before readiness. These hard hook modes correspond to --check-issue-title, --check-pr-title, --check-pr-labels, and --check-merge-message."}}"#
}

fn set_session_start_timeout(
    plugin_root: &std::path::Path,
    timeout: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["timeout"] = serde_json::json!(timeout);
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;
    Ok(())
}

fn validate_hooks_with_deadline(
    plugin_root: &std::path::Path,
    deadline: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-hooks",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
        if start.elapsed() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("validator did not return before the test deadline".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    support::copy_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}
