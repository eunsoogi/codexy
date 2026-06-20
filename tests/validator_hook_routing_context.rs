use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn session_start_context_includes_codegraph_lsp_evidence_requirements()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");

    let hook_output = Command::new(&script_path).arg("SessionStart").output()?;
    assert!(
        hook_output.status.success(),
        "hook script should emit context successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&hook_output.stdout),
        String::from_utf8_lossy(&hook_output.stderr)
    );
    let hook_json: serde_json::Value = serde_json::from_slice(&hook_output.stdout)?;
    let context = hook_json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("hook output should include additional context")?;

    for required in [
        "codegraph MCP before direct file reads",
        "include codegraph findings",
        "Use Codexy LSP",
        "lsp_status",
        "unavailable/not applicable evidence",
    ] {
        assert!(
            context.contains(required),
            "SessionStart context missing required fragment: {required}"
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_session_start_context_without_codegraph_lsp_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    for (needle, expected) in [
        (
            "codegraph MCP before direct file reads",
            "SessionStart routing context must require codegraph evidence",
        ),
        (
            "Use Codexy LSP",
            "SessionStart routing context must require LSP evidence",
        ),
        (
            "unavailable/not applicable evidence",
            "SessionStart routing context must require unavailable-tool fallback evidence",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
        let script = std::fs::read_to_string(&script_path)?.replace(needle, "");
        std::fs::write(&script_path, script)?;

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args([
                "--plugin-root",
                plugin_root.to_str().ok_or("plugin root path")?,
                "--check-hooks",
            ])
            .output()?;
        assert!(
            !output.status.success(),
            "validator should reject SessionStart routing context missing {needle:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    support::copy_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}
