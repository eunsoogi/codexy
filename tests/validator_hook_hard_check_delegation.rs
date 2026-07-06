use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn validator_cli_rejects_broken_hard_hook_delegation() -> Result<(), Box<dyn std::error::Error>> {
    for script in [
        "codexy-issue-title-check.sh",
        "codexy-pr-title-check.sh",
        "codexy-pr-label-check.sh",
        "codexy-merge-message-check.sh",
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        let script_path = plugin_root.join("hooks").join(script);
        let script_text = std::fs::read_to_string(&script_path)?;
        let broken = script_text
            .lines()
            .map(|line| {
                if line.contains("codexy-readiness-guard.sh") {
                    "exit 0"
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&script_path, format!("{broken}\n"))?;

        let output = validate_hooks(&plugin_root)?;
        assert!(
            !output.status.success(),
            "validator should reject broken hard-mode delegation for {script}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("hard-mode delegation failed"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_unsafe_sourced_hard_hook_helper() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    std::fs::write(
        plugin_root.join("hooks/codexy-readiness-guard-json.sh"),
        "#!/bin/sh\ntouch $HOME/.codex/review\n",
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject unsafe sourced hard hook helpers"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("codexy-readiness-guard-json.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_redirecting_sourced_hard_hook_helper()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let helper_path = plugin_root.join("hooks/codexy-readiness-guard-json.sh");
    let helper_text = std::fs::read_to_string(&helper_path)?;
    let state_path = temp.path().join("codexy-hook-state");
    std::fs::write(
        &helper_path,
        format!(
            "{helper_text}\nprintf '%s\\n' state > \"{}\"\n",
            state_path.display()
        ),
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject redirecting sourced hard hook helpers"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("codexy-readiness-guard-json.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_awk_closing_line_redirect_in_sourced_helper()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let helper_path = plugin_root.join("hooks/codexy-readiness-guard-json.sh");
    let helper_text = std::fs::read_to_string(&helper_path)?;
    let state_path = temp.path().join("codexy-hook-state");
    let mut replaced = false;
    let rewritten = helper_text
        .lines()
        .map(|line| {
            if !replaced && line.trim() == "'" {
                replaced = true;
                format!("' > \"{}\"", state_path.display())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        replaced,
        "expected sourced helper to contain an awk closing line"
    );
    std::fs::write(&helper_path, format!("{rewritten}\n"))?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject sourced helper awk closing-line redirects"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("codexy-readiness-guard-json.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_hard_hooks_that_reject_valid_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    for (script, expected_error) in [
        (
            "codexy-issue-title-check.sh",
            "issue title must not use Conventional Commit style",
        ),
        (
            "codexy-pr-title-check.sh",
            "PR title must use Conventional Commit style",
        ),
        (
            "codexy-pr-label-check.sh",
            "PR labels missing label application evidence",
        ),
        (
            "codexy-merge-message-check.sh",
            "merge commit subject must use Conventional Commit style",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        replace_guard_delegation_with_failure(&plugin_root, script, expected_error)?;

        let output = validate_hooks(&plugin_root)?;
        assert!(
            !output.status.success(),
            "validator should reject hard hooks that fail valid inputs for {script}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("hard-mode delegation failed"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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

fn replace_guard_delegation_with_failure(
    plugin_root: &std::path::Path,
    script: &str,
    expected_error: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let script_path = plugin_root.join("hooks").join(script);
    let script_text = std::fs::read_to_string(&script_path)?;
    let broken = script_text
        .lines()
        .map(|line| {
            if line.contains("codexy-readiness-guard.sh") {
                format!("printf '%s\\n' 'error: {expected_error}'; exit 1")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&script_path, format!("{broken}\n"))?;
    Ok(())
}

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    support::copy_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}
