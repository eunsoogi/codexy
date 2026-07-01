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
        "codegraph unavailable/uncallable fallback evidence",
        "registered-but-uncallable/unavailable-tool evidence",
        "Use Codexy LSP",
        "lsp_status",
        "unavailable/not applicable evidence",
        "$dreaming",
        "compacted or resumed context hygiene",
        "codexy-readiness-guard.sh",
        "--check-pr-title",
        "--check-merge-message",
        "--expected-pr",
    ] {
        assert!(
            context.contains(required),
            "SessionStart context missing required fragment: {required}"
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_session_start_command_with_routing_path_substring()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh.disabled");
    std::fs::write(&script_path, "#!/bin/sh\nprintf '%s\\n' not-routing\n")?;
    set_session_start_hook_command(
        &plugin_root,
        "\"${PLUGIN_ROOT}/hooks/codexy-routing-context.sh.disabled\" SessionStart",
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject SessionStart commands that only contain the routing script path"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("SessionStart hook command must run hooks/codexy-routing-context.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_session_start_command_with_different_static_args()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    set_session_start_hook_command(
        &plugin_root,
        "\"${PLUGIN_ROOT}/hooks/codexy-routing-context.sh\" UserPromptSubmit",
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject SessionStart hooks configured with a different invocation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("SessionStart hook command must invoke SessionStart exactly"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_session_start_context_that_only_mentions_requirements_in_comments()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    let mut script = std::fs::read_to_string(&script_path)?;
    for required in required_context_fragments() {
        script = script.replace(source_context_fragment(required), "");
        script.push_str(&format!("\n# {required}\n"));
    }
    std::fs::write(&script_path, script)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject source-only routing context requirements"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("emitted additionalContext"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_session_start_context_without_codegraph_lsp_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    for (needle, expected) in [
        (
            "codegraph MCP before direct file reads",
            "must require codegraph evidence",
        ),
        ("Use Codexy LSP", "must require LSP evidence"),
        (
            "registered-but-uncallable/unavailable-tool evidence",
            "must require codegraph fallback evidence",
        ),
        (
            "codegraph unavailable/uncallable fallback evidence",
            "must require codegraph fallback evidence",
        ),
        (
            "unavailable/not applicable evidence",
            "must require LSP fallback evidence",
        ),
        ("$dreaming", "must require dreaming hygiene"),
        (
            "compacted or resumed context hygiene",
            "must require dreaming hygiene",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
        let script =
            std::fs::read_to_string(&script_path)?.replace(source_context_fragment(needle), "");
        std::fs::write(&script_path, script)?;

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args([
                "--plugin-root",
                plugin_root.to_str().ok_or("plugin root path")?,
                "--check-hooks",
            ])
            .output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "validator should reject SessionStart routing context missing {needle:?}"
        );
        assert!(
            stderr.contains("emitted additionalContext") && stderr.contains(expected),
            "unexpected stderr: {stderr}"
        );
    }
    Ok(())
}

fn required_context_fragments() -> [&'static str; 13] {
    [
        "codegraph MCP before direct file reads",
        "include codegraph findings",
        "codegraph unavailable/uncallable fallback evidence",
        "registered-but-uncallable/unavailable-tool evidence",
        "Use Codexy LSP",
        "lsp_status",
        "unavailable/not applicable evidence",
        "$dreaming",
        "compacted or resumed context hygiene",
        "codexy-readiness-guard.sh",
        "--check-pr-title",
        "--check-merge-message",
        "--expected-pr",
    ]
}

fn source_context_fragment(fragment: &str) -> &str {
    match fragment {
        "$dreaming" => "\\$dreaming",
        _ => fragment,
    }
}

fn set_session_start_hook_command(
    plugin_root: &std::path::Path,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["command"] = serde_json::json!(command);
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;
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
