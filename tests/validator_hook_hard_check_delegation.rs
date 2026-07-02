use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn validator_cli_rejects_broken_hard_hook_delegation() -> Result<(), Box<dyn std::error::Error>> {
    for script in [
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

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    support::copy_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}
