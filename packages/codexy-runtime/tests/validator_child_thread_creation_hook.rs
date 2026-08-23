use std::{io::Write as _, process::Stdio};

use crate::support::FixtureCommand as Command;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TOOL: &str = "codex_app__create_thread";
const LAUNCHER: &str = "codexy-child-thread-creation.sh";

#[test]
fn exact_wave_zero_omitted_field_call_is_rejected_before_mutation() -> TestResult {
    let input = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": TOOL,
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
    assert_hook(&input, true)
}

#[test]
fn explicit_pairs_are_admitted_without_route_claims() -> TestResult {
    let cases = [
        ("generic default", json!({"model":"gpt-5.6-luna","thinking":"max"})),
        ("explicit Terra", json!({"model":"gpt-5.6-terra","thinking":"high"})),
        ("explicit Sol", json!({"model":"gpt-5.6-sol","thinking":"medium"})),
    ];

    for (label, tool_input) in cases {
        assert!(!hook_denied(&pre_tool_input(tool_input))?, "{label}");
    }
    Ok(())
}

#[test]
fn malformed_caller_route_metadata_is_not_a_hook_authority() -> TestResult {
    let input = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":TOOL,
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
        let input = pre_tool_input(tool_input);
        assert_hook(&input, true)?;
    }
    Ok(())
}

#[test]
fn both_preventive_events_apply_admission_before_mutation() -> TestResult {
    let input = json!({
        "tool_name":TOOL,
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

fn pre_tool_input(tool_input: Value) -> Value {
    json!({"hook_event_name":"PreToolUse","tool_name":TOOL,"tool_input":tool_input})
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
