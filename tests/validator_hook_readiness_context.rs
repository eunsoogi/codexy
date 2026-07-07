use std::process::{Command, Output};
use std::time::{Duration, Instant};

#[allow(unused)]
mod support;

#[test]
fn validator_cli_bounds_readiness_context_execution() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    set_readiness_script(&plugin_root, "#!/bin/sh\nsleep 30\n")?;

    let output = validate_hooks_with_deadline(&plugin_root, Duration::from_secs(6))?;
    assert!(
        !output.status.success(),
        "validator should reject a readiness hook that exceeds its timeout"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("timed out"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_readiness_context_invalid_json() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    set_readiness_script(&plugin_root, "#!/bin/sh\nprintf '%s\\n' not-json\n")?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject invalid readiness hook JSON"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("UserPromptSubmit hook output must be JSON"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_readiness_context_wrong_event() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    set_readiness_script(
        &plugin_root,
        r#"#!/bin/sh
printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"ready"}}'
"#,
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject readiness hook output with the wrong event"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("hook output must set hookSpecificOutput.hookEventName to UserPromptSubmit"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_readiness_context_without_additional_context()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    set_readiness_script(
        &plugin_root,
        r#"#!/bin/sh
printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit"}}'
"#,
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject readiness hook output without additionalContext"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("hook output must emit non-empty hookSpecificOutput.additionalContext"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_readiness_context_without_label_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    set_readiness_script(
        &plugin_root,
        r#"#!/bin/sh
printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"codexy-issue-title-check.sh --check-issue-title PR label readiness enforcement (#210) placeholder only"}}'
"#,
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject readiness context without label gate command"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must require PR label readiness guard"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_readiness_context_without_stacked_base_hook_guard()
-> Result<(), Box<dyn std::error::Error>> {
    for (fragment, expected) in [
        (
            "target base",
            "must require target-base hook entrypoint validation",
        ),
        (
            "hook entrypoints",
            "must require target-base hook entrypoint validation",
        ),
        (
            "available fallback",
            "must require hook fallback or mismatch defect routing",
        ),
        (
            "separate dogfood defect",
            "must require hook fallback or mismatch defect routing",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        let script_path = plugin_root.join("hooks/codexy-readiness-context.sh");
        let script = std::fs::read_to_string(&script_path)?.replace(fragment, "");
        std::fs::write(&script_path, script)?;

        let output = validate_hooks(&plugin_root)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "validator should reject readiness context missing {fragment:?}"
        );
        assert!(
            stderr.contains("emitted additionalContext") && stderr.contains(expected),
            "unexpected stderr: {stderr}"
        );
    }
    Ok(())
}

fn set_readiness_script(
    plugin_root: &std::path::Path,
    script: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(
        plugin_root.join("hooks/codexy-readiness-context.sh"),
        script,
    )?;
    Ok(())
}

fn validate_hooks(
    plugin_root: &std::path::Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-hooks",
        ])
        .output()?)
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
