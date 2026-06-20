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

fn session_start_context_json() -> &'static str {
    r#"{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"Use Codexy codegraph MCP before direct file reads; include codegraph findings; record codegraph unavailable/uncallable fallback evidence; record registered-but-uncallable/unavailable-tool evidence. Use Codexy LSP; run lsp_status; record unavailable/not applicable evidence."}}"#
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
