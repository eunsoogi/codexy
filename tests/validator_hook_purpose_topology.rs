#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[allow(unused)]
use crate::support;

const QUIET_EVENTS: &[&str] = &["SessionStart", "UserPromptSubmit"];
const HARD_CHECKS: &[&str] = &[
    "codexy-issue-title-check.sh",
    "codexy-pr-title-check.sh",
    "codexy-pr-label-check.sh",
    "codexy-merge-message-check.sh",
];

#[test]
fn packaged_hooks_are_lifecycle_quiet() -> Result<(), Box<dyn std::error::Error>> {
    let hooks = packaged_hooks()?;
    let events = hooks["hooks"]
        .as_object()
        .ok_or("hooks must be an object")?;
    for event in QUIET_EVENTS {
        assert!(
            !events.contains_key(*event),
            "packaged hooks must not register the lifecycle event {event}"
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_an_unrelated_safe_lifecycle_hook() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = lifecycle_quiet_fixture(temp.path())?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        output.status.success(),
        "validator should accept safe non-diagnostic hooks\n{}",
        output_text(&output)
    );
    Ok(())
}

#[test]
fn validator_rejects_diagnostic_lifecycle_hooks() -> Result<(), Box<dyn std::error::Error>> {
    for event in QUIET_EVENTS {
        let temp = tempfile::tempdir()?;
        let plugin_root = lifecycle_quiet_fixture(temp.path())?;
        add_diagnostic_hook(&plugin_root, event)?;

        let output = validate_hooks(&plugin_root)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "validator accepted {event}: {stderr}"
        );
        assert!(
            stderr.contains("lifecycle-quiet") && stderr.contains(event),
            "expected a clear lifecycle-quiet error for {event}, got: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn hard_checks_remain_packaged_executable_and_independently_callable()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = lifecycle_quiet_fixture(temp.path())?;
    let hooks = packaged_hooks()?;
    let hooks_text = serde_json::to_string(&hooks)?;
    for script in HARD_CHECKS {
        let path = plugin_root.join("hooks").join(script);
        assert!(
            path.is_file(),
            "missing packaged hard-check script: {script}"
        );
        #[cfg(unix)]
        assert!(
            path.metadata()?.permissions().mode() & 0o111 != 0,
            "packaged hard-check script must be executable: {script}"
        );
        assert!(
            hooks_text.find(script).is_none(),
            "hard-check script must remain callable without lifecycle registration: {script}"
        );
    }

    assert_static_success(
        &plugin_root.join("hooks/codexy-issue-title-check.sh"),
        &["--issue-title", "Keep hard checks independently callable"],
    )?;
    assert_static_success(
        &plugin_root.join("hooks/codexy-pr-title-check.sh"),
        &[
            "--pr-title",
            "fix(hooks): keep checks independently callable",
        ],
    )?;
    let pr_state = temp.path().join("labeled-pr.json");
    std::fs::write(
        &pr_state,
        r#"{"number":219,"state":"OPEN","repository":"eunsoogi/codexy","labels":[{"name":"type/fix"}],"repositoryLabels":[{"name":"type/fix"}]}"#,
    )?;
    assert_static_success(
        &plugin_root.join("hooks/codexy-pr-label-check.sh"),
        &["--pr-state-file", pr_state.to_str().ok_or("pr state path")?],
    )?;
    assert_static_success(
        &plugin_root.join("hooks/codexy-merge-message-check.sh"),
        &[
            "--expected-issue",
            "219",
            "--expected-pr",
            "220",
            "--merge-message",
            "fix(hooks): keep checks independently callable (#220)\n\nFixes #219\n",
        ],
    )?;
    Ok(())
}

fn lifecycle_quiet_fixture(
    base: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let plugin_root = base.join("codexy");
    copy_plugin(&plugin_root)?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks = read_hooks(&hooks_path)?;
    let events = hooks["hooks"]
        .as_object_mut()
        .ok_or("hooks must be an object")?;
    for event in QUIET_EVENTS {
        events.remove(*event);
    }
    events.insert("PostToolUse".to_string(), safe_post_tool_use_hook());
    std::fs::write(hooks_path, serde_json::to_string_pretty(&hooks)?)?;
    Ok(plugin_root)
}

fn add_diagnostic_hook(
    plugin_root: &std::path::Path,
    event: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks = read_hooks(&hooks_path)?;
    hooks["hooks"]
        .as_object_mut()
        .ok_or("hooks must be an object")?
        .insert(event.to_string(), serde_json::json!([{
            "matcher": "*",
            "hooks": [{
                "type": "command",
                "command": "\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\" --issue-title Valid",
                "timeout": 3
            }]
        }]));
    std::fs::write(hooks_path, serde_json::to_string_pretty(&hooks)?)?;
    Ok(())
}

fn safe_post_tool_use_hook() -> serde_json::Value {
    serde_json::json!([{
        "matcher": "*",
        "hooks": [{
            "type": "command",
            "command": "\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\" --issue-title Valid",
            "timeout": 3
        }]
    }])
}

fn packaged_hooks() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    read_hooks(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/hooks/hooks.json"),
    )
}

fn read_hooks(path: &std::path::Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn assert_static_success(
    script: &std::path::Path,
    args: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(script).args(args).output()?;
    assert!(
        output.status.success(),
        "{} static mode failed:\n{}",
        script.display(),
        output_text(&output)
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

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
