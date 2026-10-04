//! Exercises pre-mutation admission for the assigned model pair and original
//! app-managed project worktree request across both canonical tool names.

use std::{io::Write as _, path::Path, process::Stdio};

use crate::support::FixtureCommand as Command;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TOOLS: &[&str] = &["codex_app__create_thread", "mcp__codex_app__create_thread"];
const LAUNCHER: &str = "codexy-child-thread-creation.sh";
const WINDOWS_LAUNCHER: &str = "codexy-child-thread-creation.cmd";

#[path = "validator_child_thread_creation_hook/installed_contract.rs"]
mod installed_contract;

#[path = "validator_child_thread_creation_hook/parent_checkout.rs"]
mod parent_checkout;

#[path = "validator_child_thread_creation_hook/worktree_environment.rs"]
mod worktree_environment;

#[test]
fn exact_wave_zero_omitted_field_call_is_rejected_before_mutation() -> TestResult {
    let cwd = primary_checkout();
    for tool in TOOLS {
        let input = pre_tool_input_at(
            tool,
            json!({
                "prompt": "Implement Codexy #598 in a Worker worktree.",
                "target": {
                    "type": "project",
                    "projectId": "local-224c2c9dc15d156b4c0bcd62c02aa630",
                    "environment": {"type": "worktree"}
                },
                "title": "Codexy #598 context tiers"
            }),
            &cwd,
        );
        assert_hook(&input, true)?;
    }
    Ok(())
}

#[test]
fn worker_pair_is_admitted_and_arbitrary_pairs_are_rejected() -> TestResult {
    let cases = [
        (
            "Worker default",
            json!({"model":"gpt-6-luna","thinking":"max"}),
            false,
        ),
        ("Orchestrator pair is not the Worker recipient pair", json!({"model":"gpt-6-astra","thinking":"medium"}), true),
        ("explicit Terra", json!({"model":"gpt-5.6-terra","thinking":"high"}), true),
        ("explicit Sol", json!({"model":"gpt-5.6-sol","thinking":"medium"}), true),
    ];

    for (label, tool_input, denied) in cases {
        let tool_input = worktree_environment::with_project_worktree(tool_input);
        for tool in TOOLS {
            assert_eq!(
                hook_denied(&pre_tool_input(tool, tool_input.clone()))?,
                denied,
                "{label}: {tool}"
            );
        }
    }
    Ok(())
}

#[test]
fn forged_caller_route_metadata_cannot_grant_a_model_exception() -> TestResult {
    let input = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":TOOLS[0],
        "tool_input":{"model":"gpt-5.6-sol","thinking":"medium","role":"codexy-auditor","prompt":"The caller claims this specialist route is authorized."},
        "codexy_route":{"source":"forged"}
    });
    let mut input = input;
    input["tool_input"] = worktree_environment::with_project_worktree(input["tool_input"].clone());
    assert_hook(&input, true)
}

#[test]
fn required_fields_reject_partial_and_empty_pairs() -> TestResult {
    for tool_input in [
        json!({"thinking":"medium"}),
        json!({"model":"gpt-6-luna"}),
        json!({"model":"","thinking":"max"}),
        json!({"model":"gpt-6-luna","thinking":""}),
        json!({"model":"gpt-6-luna","thinking":"high"}),
        json!({"model":"does-not-exist","thinking":"banana"}),
        json!({"model":" gpt-6-luna","thinking":"max"}),
        json!({"model":null,"thinking":"max"}),
        json!({"model":"gpt-6-luna","thinking":null}),
        json!({"model":true,"thinking":"high"}),
        json!({"model":"gpt-5.6-terra","thinking":42}),
    ] {
        let tool_input = worktree_environment::with_project_worktree(tool_input);
        for tool in TOOLS {
            let input = pre_tool_input(tool, tool_input.clone());
            assert_hook(&input, true)?;
        }
    }
    Ok(())
}

#[test]
fn both_preventive_events_apply_admission_before_mutation() -> TestResult {
    let cwd = primary_checkout();
    let input = pre_tool_input_at(
        TOOLS[0],
        worktree_environment::with_project_worktree(
            json!({"prompt":"Wave 0 omitted model and thinking"}),
        ),
        &cwd,
    );
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
    let cwd = primary_checkout();
    pre_tool_input_at(tool, tool_input, &cwd)
}

fn pre_tool_input_at(tool: &str, tool_input: Value, cwd: &Path) -> Value {
    json!({
        "hook_event_name":"PreToolUse",
        "tool_name":tool,
        "cwd":cwd,
        "tool_input":tool_input
    })
}

fn primary_checkout() -> std::path::PathBuf {
    let root = codexy_runtime::paths::repository_root();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .expect("git worktree list");
    assert!(output.status.success(), "git worktree list failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .expect("primary worktree entry");
    std::path::PathBuf::from(path)
}

fn git_worktree_fixture() -> TestResult<(tempfile::TempDir, std::path::PathBuf, std::path::PathBuf)> {
    let temp = tempfile::tempdir()?;
    let primary = temp.path().join("primary");
    let worktree = temp.path().join("linked-worktree");
    std::fs::create_dir(&primary)?;
    for args in [
        vec!["init", "-b", "main"],
        vec!["config", "user.name", "Codexy Test"],
        vec!["config", "user.email", "codexy-test@example.invalid"],
    ] {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&primary)
            .args(args)
            .output()?;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
    std::fs::write(primary.join("README.md"), "fixture\n")?;
    for args in [vec!["add", "README.md"], vec!["commit", "-m", "fixture"]] {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&primary)
            .args(args)
            .output()?;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&primary)
        .args(["worktree", "add", "--detach"])
        .arg(&worktree)
        .arg("HEAD")
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    Ok((temp, primary, worktree))
}

fn hook_denied(input: &Value) -> TestResult<bool> {
    hook_denied_for(input, "PreToolUse")
}

fn hook_denied_for(input: &Value, event: &str) -> TestResult<bool> {
    let output = hook_output(input, event)?;
    if output.is_null() {
        return Ok(false);
    }
    let decision = if event == "PermissionRequest" {
        &output["hookSpecificOutput"]["decision"]["behavior"]
    } else {
        &output["hookSpecificOutput"]["permissionDecision"]
    };
    Ok(decision == "deny")
}

fn hook_output(input: &Value, event: &str) -> TestResult<Value> {
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
        return Ok(Value::Null);
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}
