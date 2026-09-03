use std::{io::Write as _, process::Stdio};

use crate::support::FixtureCommand as Command;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TOOLS: &[&str] = &["codex_app__create_thread", "mcp__codex_app__create_thread"];
const LAUNCHER: &str = "codexy-child-thread-creation.sh";
const WINDOWS_LAUNCHER: &str = "codexy-child-thread-creation.cmd";

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

#[cfg(windows)]
#[test]
fn native_windows_child_launcher_runtime_failure_emits_valid_permission_denial() -> TestResult {
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

#[test]
fn exact_wave_zero_omitted_field_call_is_rejected_before_mutation() -> TestResult {
    for tool in TOOLS {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": tool,
            "tool_input": {
                "prompt": "Implement Codexy #598 in a child worktree.",
                "target": {
                    "type": "project",
                    "projectId": "local-224c2c9dc15d156b4c0bcd62c02aa630",
                    "environment": {"type": "worktree"}
                },
                "title": "Codexy #598 context tiers"
            }
        });
        assert_hook(&input, true)?;
    }
    Ok(())
}

#[test]
fn explicit_pairs_are_admitted_without_route_claims() -> TestResult {
    let cases = [
        ("generic default", json!({"model":"gpt-5.6-luna","thinking":"max"})),
        ("explicit Terra", json!({"model":"gpt-5.6-terra","thinking":"high"})),
        ("explicit Sol", json!({"model":"gpt-5.6-sol","thinking":"medium"})),
    ];

    for (label, tool_input) in cases {
        for tool in TOOLS {
            assert!(
                !hook_denied(&pre_tool_input(tool, tool_input.clone()))?,
                "{label}: {tool}"
            );
        }
    }
    Ok(())
}

#[test]
fn malformed_caller_route_metadata_is_not_a_hook_authority() -> TestResult {
    let input = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":TOOLS[0],
        "tool_input":{"model":"gpt-5.6-sol","thinking":"medium"},
        "codexy_route":{"source":"forged"}
    });
    assert_hook(&input, false)
}

#[test]
fn required_fields_reject_partial_and_empty_pairs() -> TestResult {
    for tool_input in [
        json!({"thinking":"medium"}),
        json!({"model":"gpt-5.6-luna"}),
        json!({"model":"","thinking":"max"}),
        json!({"model":"gpt-5.6-luna","thinking":""}),
        json!({"model":null,"thinking":"max"}),
        json!({"model":"gpt-5.6-luna","thinking":null}),
        json!({"model":true,"thinking":"high"}),
        json!({"model":"gpt-5.6-terra","thinking":42}),
    ] {
        for tool in TOOLS {
            let input = pre_tool_input(tool, tool_input.clone());
            assert_hook(&input, true)?;
        }
    }
    Ok(())
}

#[test]
fn both_preventive_events_apply_admission_before_mutation() -> TestResult {
    let input = json!({
        "tool_name":TOOLS[0],
        "tool_input":{"prompt":"Wave 0 omitted model and thinking"}
    });
    for event in ["PermissionRequest", "PreToolUse"] {
        let mut event_input = input.clone();
        event_input["hook_event_name"] = json!(event);
        assert!(hook_denied_for(&event_input, event)?);
    }
    Ok(())
}

fn assert_hook(input: &Value, denied: bool) -> TestResult {
    assert_eq!(hook_denied(input)?, denied, "{input}");
    Ok(())
}

fn pre_tool_input(tool: &str, tool_input: Value) -> Value {
    json!({"hook_event_name":"PreToolUse","tool_name":tool,"tool_input":tool_input})
}

fn hook_denied(input: &Value) -> TestResult<bool> {
    hook_denied_for(input, "PreToolUse")
}

fn hook_denied_for(input: &Value, event: &str) -> TestResult<bool> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let launcher = root.join("hooks").join(LAUNCHER);
    assert!(
        launcher.is_file(),
        "#660 production admission launcher is missing: {}",
        launcher.display()
    );
    let mut child = Command::new(&launcher)
        .arg(event)
        .env("PLUGIN_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "hook failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(
        output.stderr.is_empty(),
        "unexpected hook stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if output.stdout.is_empty() {
        return Ok(false);
    }
    let output: Value = serde_json::from_slice(&output.stdout)?;
    let decision = if event == "PermissionRequest" {
        &output["hookSpecificOutput"]["decision"]["behavior"]
    } else {
        &output["hookSpecificOutput"]["permissionDecision"]
    };
    Ok(decision == "deny")
}
