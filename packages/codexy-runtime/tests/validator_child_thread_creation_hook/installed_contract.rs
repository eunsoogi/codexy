//! Checks installed hook contracts, including actionable delegated-route diagnostics.

use super::*;

#[path = "model_feedback.rs"]
mod model_feedback;

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
    assert_eq!(
        denial["hookSpecificOutput"]["hookEventName"],
        "PermissionRequest"
    );
    assert_eq!(denial["hookSpecificOutput"]["decision"]["behavior"], "deny");
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
fn worker_creation_diagnostics_describe_both_supported_pairs_and_orchestrator_repair() -> TestResult {
    let cases = [
        ("missing settings", json!({}), "MISSING_ROUTE_FIELDS"),
        (
            "missing model",
            json!({"thinking":"max"}),
            "MISSING_MODEL",
        ),
        (
            "missing thinking",
            json!({"model":"gpt-6-luna"}),
            "MISSING_THINKING",
        ),
        (
            "unsupported model",
            json!({"model":"gpt-5.6-sol","thinking":"medium"}),
            "UNSUPPORTED_MODEL",
        ),
        (
            "crossed pair",
            json!({"model":"gpt-6.1-terra","thinking":"max"}),
            "UNSUPPORTED_THINKING",
        ),
        (
            "unsupported Sol/Medium Worker pair",
            json!({"model":"gpt-6.1-sol","thinking":"medium"}),
            "UNSUPPORTED_MODEL",
        ),
    ];

    for (label, settings, diagnostic) in cases {
        let tool_input = worktree_environment::with_project_worktree(settings);
        for tool in TOOLS {
            for event in ["PreToolUse", "PermissionRequest"] {
                let mut input = pre_tool_input(tool, tool_input.clone());
                input["hook_event_name"] = json!(event);
                let output = hook_output(&input, event)?;
                let reason = if event == "PermissionRequest" {
                    output["hookSpecificOutput"]["decision"]["message"].as_str()
                } else {
                    output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
                }
                .ok_or("Worker creation diagnostic")?;
                assert!(reason.contains(diagnostic), "{label} {event}: {reason}");
                for pair in [
                    "model='gpt-6-luna' and thinking='max'",
                    "model='gpt-6.1-terra' and thinking='medium'",
                ] {
                    assert!(
                        reason.contains(pair),
                        "{label} {event} omitted {pair}: {reason}"
                    );
                }
                assert_worker_selection_guidance(reason, &format!("{label} {event}"));
            }
        }
    }
    Ok(())
}

#[test]
fn invalid_worktree_diagnostic_assigns_the_project_target_to_the_orchestrator() -> TestResult {
    for tool in TOOLS {
        for event in ["PreToolUse", "PermissionRequest"] {
            let mut input = pre_tool_input(
                tool,
                json!({"model":"gpt-6-luna","thinking":"max"}),
            );
            input["hook_event_name"] = json!(event);
            let output = hook_output(&input, event)?;
            let reason = if event == "PermissionRequest" {
                output["hookSpecificOutput"]["decision"]["message"].as_str()
            } else {
                output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
            }
            .ok_or("worktree creation diagnostic")?;
            assert!(reason.contains("APP_WORKTREE_REQUIRED"), "{event}: {reason}");
            assert!(reason.contains("target.environment.type='worktree'"), "{event}: {reason}");
            assert_worker_selection_guidance(reason, event);
        }
    }
    Ok(())
}

#[test]
fn malformed_creation_input_keeps_the_same_orchestrator_repair_guidance() -> TestResult {
    for tool in TOOLS {
        let input = pre_tool_input(tool, Value::Null);
        let output = hook_output(&input, "PreToolUse")?;
        let reason = output["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .ok_or("malformed creation diagnostic")?;
        assert!(reason.contains("MISSING_ROUTE_FIELDS"), "{reason}");
        assert_worker_selection_guidance(reason, "malformed creation input");
    }
    Ok(())
}

fn assert_worker_selection_guidance(reason: &str, label: &str) {
    for requirement in [
        "already-authorized Worker assignment",
        "reuse any already-selected Worker pair",
        "if none is selected, classify the task evidence",
        "canonical child-routing policy",
        "ordinary work uses model='gpt-6-luna' and thinking='max'",
        "only when the task evidence meets the hard-work criteria",
        "MUST NOT ask the user to select or reconfirm model, thinking, or worktree",
        "MUST correct only the identified fields in the original create_thread request",
        "continue creating and assigning the Worker",
        "inspect that exact result to distinguish a newly identified input error",
        "For a host restriction, report the exact limitation",
        "does not create authority for a new task",
    ] {
        assert!(
            reason.contains(requirement),
            "{label} omitted {requirement}: {reason}"
        );
    }
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
    assert_eq!(
        denial["hookSpecificOutput"]["hookEventName"],
        "PermissionRequest"
    );
    assert_eq!(denial["hookSpecificOutput"]["decision"]["behavior"], "deny");
    Ok(())
}
