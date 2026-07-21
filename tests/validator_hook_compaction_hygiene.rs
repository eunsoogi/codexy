use std::process::Command;

#[allow(unused)]
use crate::support;

#[test]
fn validator_cli_rejects_session_start_matcher_without_compact_resume()
-> Result<(), Box<dyn std::error::Error>> {
    for matcher in ["startup|clear", "   "] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        let hooks_path = plugin_root.join("hooks/hooks.json");
        let mut hooks_config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
        hooks_config["hooks"]["SessionStart"][0]["matcher"] = serde_json::json!(matcher);
        std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

        let output = validate_hooks(&plugin_root)?;
        assert!(
            !output.status.success(),
            "validator should reject SessionStart matcher {matcher:?} without compact and resume"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("SessionStart.matcher must include resume and compact"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_session_start_regex_matchers_covering_compact_resume()
-> Result<(), Box<dyn std::error::Error>> {
    for matcher in ["^(startup|resume|clear|compact)$", ".*"] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        let hooks_path = plugin_root.join("hooks/hooks.json");
        let mut hooks_config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
        hooks_config["hooks"]["SessionStart"][0]["matcher"] = serde_json::json!(matcher);
        std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

        let output = validate_hooks(&plugin_root)?;
        assert!(
            output.status.success(),
            "validator should accept SessionStart regex matcher {matcher:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_session_start_wildcard_matchers_covering_compact_resume()
-> Result<(), Box<dyn std::error::Error>> {
    for matcher in [Some("*"), Some(""), None] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        let hooks_path = plugin_root.join("hooks/hooks.json");
        let mut hooks_config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
        match matcher {
            Some(matcher) => {
                hooks_config["hooks"]["SessionStart"][0]["matcher"] = serde_json::json!(matcher);
            }
            None => {
                hooks_config["hooks"]["SessionStart"][0]
                    .as_object_mut()
                    .ok_or("SessionStart hook group must be an object")?
                    .remove("matcher");
            }
        }
        std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

        let output = validate_hooks(&plugin_root)?;
        assert!(
            output.status.success(),
            "validator should accept SessionStart wildcard matcher {matcher:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_non_string_matcher_when_present() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["PostToolUse"] = serde_json::json!([{ "matcher": 123, "hooks": [{
        "type": "command",
        "command": "\"${PLUGIN_ROOT}/hooks/codexy-routing-context.sh\" SessionStart",
        "timeout": 3
    }]}]);
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject present non-string hook matchers"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("PostToolUse.matcher must be a non-empty string when present"),
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
