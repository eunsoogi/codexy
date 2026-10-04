use super::*;

#[test]
fn windows_permission_request_runtime_failure_fallback_is_valid_json() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let source = std::fs::read_to_string(root.join("hooks").join(WINDOWS_LAUNCHER))?;
    let fallback = source
        .lines()
        .find(|line| {
            line.starts_with("echo {\"hookSpecificOutput\"")
                && line.contains("\"hookEventName\":\"PermissionRequest\"")
        })
        .ok_or("PermissionRequest fallback")?;
    let denial: Value = serde_json::from_str(fallback.strip_prefix("echo ").ok_or("echo")?)?;
    assert_eq!(denial["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
    assert_eq!(
        denial["hookSpecificOutput"]["decision"]["behavior"],
        "deny"
    );
    Ok(())
}

#[test]
fn installed_matcher_covers_both_canonical_create_thread_tool_names() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(root.join("hooks.json"))?)?;
    let matcher = hooks["hooks"]["PreToolUse"][1]["matcher"]
        .as_str()
        .ok_or("create_thread matcher")?;
    let matcher = regex::Regex::new(matcher)?;
    for tool in TOOLS {
        assert!(matcher.is_match(tool), "matcher misses {tool}");
    }
    for tool in [
        "mcp__codex_app__send_message_to_thread",
        "codex_app__create_thread_extra",
        "mcp__codex_app__create_thread_extra",
    ] {
        assert!(!matcher.is_match(tool), "matcher overmatches {tool}");
    }
    Ok(())
}

#[test]
fn installed_thread_delivery_matcher_covers_both_canonical_tool_names() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(root.join("hooks.json"))?)?;
    let matcher = hooks["hooks"]["PreToolUse"][0]["matcher"]
        .as_str()
        .ok_or("send_message_to_thread matcher")?;
    let matcher = regex::Regex::new(matcher)?;
    for prefix in ["codex_app__", "mcp__codex_app__"] {
        let tool = format!("{prefix}send_message_to_thread");
        assert!(matcher.is_match(&tool), "matcher misses {tool}");
    }
    assert!(!matcher.is_match("mcp__codex_app__create_thread"));
    Ok(())
}

#[cfg(windows)]
#[test]
fn native_windows_worker_launcher_runtime_failure_emits_valid_permission_denial() -> TestResult {
    let temp = tempfile::tempdir()?;
    let launcher = temp.path().join(WINDOWS_LAUNCHER);
    std::fs::copy(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/hooks")
            .join(WINDOWS_LAUNCHER),
        &launcher,
    )?;
    std::fs::write(
        temp.path().join("codexy-child-thread-creation.py"),
        "import sys\nsys.exit(1)\n",
    )?;
    let output = std::process::Command::new("cmd")
        .arg("/d")
        .arg("/c")
        .arg(&launcher)
        .arg("PermissionRequest")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let denial: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(denial["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
    assert_eq!(denial["hookSpecificOutput"]["decision"]["behavior"], "deny");
    Ok(())
}
