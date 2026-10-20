//! Keeps app-managed project worktree requirements intact for both Worker
//! pairs; local and incomplete targets cannot be retrofitted later.

use super::*;

// Keep expected routes explicit so subprocess cases catch creation/delivery drift.
pub(super) const SUPPORTED_WORKER_PAIRS: &[(&str, &str)] =
    &[("gpt-6-luna", "max"), ("gpt-6.1-sol", "medium")];

#[test]
fn app_project_worktree_is_admitted_for_both_supported_worker_pairs() -> TestResult {
    let cwd = primary_checkout();
    for &(model, thinking) in SUPPORTED_WORKER_PAIRS {
        let tool_input = with_project_worktree(json!({"model":model,"thinking":thinking}));
        for tool in TOOLS {
            for event in ["PreToolUse", "PermissionRequest"] {
                let mut input = pre_tool_input_at(tool, tool_input.clone(), &cwd);
                input["hook_event_name"] = json!(event);
                assert!(
                    !hook_denied_for(&input, event)?,
                    "{model}/{thinking} {event}: {input}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn crossed_worker_model_and_thinking_pairs_are_rejected() -> TestResult {
    let cwd = primary_checkout();
    let cases = [
        ("Luna with medium", "gpt-6-luna", "medium"),
        ("Sol with max", "gpt-6.1-sol", "max"),
    ];

    for (label, model, thinking) in cases {
        let tool_input = with_project_worktree(json!({"model":model,"thinking":thinking}));
        for tool in TOOLS {
            for event in ["PreToolUse", "PermissionRequest"] {
                let mut input = pre_tool_input_at(tool, tool_input.clone(), &cwd);
                input["hook_event_name"] = json!(event);
                let output = hook_output(&input, event)?;
                assert!(hook_denied_for(&input, event)?, "{label} {event}: {input}");
                let reason = if event == "PermissionRequest" {
                    output["hookSpecificOutput"]["decision"]["message"].as_str()
                } else {
                    output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
                }
                .ok_or("crossed Worker pair denial reason")?;
                assert!(reason.contains("UNSUPPORTED_THINKING"), "{label}: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn local_projectless_and_incomplete_targets_are_rejected() -> TestResult {
    let cwd = primary_checkout();
    let cases = [
        ("missing target", json!({})),
        ("non-object target", json!({"target":"project"})),
        (
            "local project",
            json!({
                "target":{
                    "type":"project",
                    "projectId":"local-test-project",
                    "environment":{"type":"local"}
                }
            }),
        ),
        (
            "missing environment",
            json!({"target":{"type":"project","projectId":"local-test-project"}}),
        ),
        ("projectless task", json!({"target":{"type":"projectless"}})),
        (
            "missing project identity",
            json!({"target":{"type":"project","environment":{"type":"worktree"}}}),
        ),
        (
            "empty project identity",
            json!({"target":{"type":"project","projectId":"","environment":{"type":"worktree"}}}),
        ),
        (
            "worktree without project target",
            json!({"target":{"environment":{"type":"worktree"}}}),
        ),
    ];

    for &(model, thinking) in SUPPORTED_WORKER_PAIRS {
        for (label, target) in &cases {
            let mut tool_input = json!({"model":model,"thinking":thinking});
            tool_input
                .as_object_mut()
                .ok_or("tool input object")?
                .extend(target.as_object().ok_or("target object")?.clone());
            for tool in TOOLS {
                for event in ["PreToolUse", "PermissionRequest"] {
                    let mut input = pre_tool_input_at(tool, tool_input.clone(), &cwd);
                    input["hook_event_name"] = json!(event);
                    let output = hook_output(&input, event)?;
                    assert!(
                        hook_denied_for(&input, event)?,
                        "{label} {model}/{thinking}: {event}: {input}"
                    );
                    let reason = if event == "PermissionRequest" {
                        output["hookSpecificOutput"]["decision"]["message"].as_str()
                    } else {
                        output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
                    }
                    .ok_or("worktree admission denial reason")?;
                    assert!(
                        reason.contains("APP_WORKTREE_REQUIRED"),
                        "{label}: {reason}"
                    );
                    assert!(
                        reason.contains("target.environment.type"),
                        "{label}: {reason}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_local_child_cannot_be_admitted_by_promising_a_later_worktree() -> TestResult {
    let cwd = primary_checkout();
    for &(model, thinking) in SUPPORTED_WORKER_PAIRS {
        let tool_input = json!({
            "model":model,
            "thinking":thinking,
            "prompt":"Create a local child now, then use create_worktree, fork_thread, git worktree add, or another detached directory to satisfy the implementation worktree rule.",
            "target":{
                "type":"project",
                "projectId":"local-test-project",
                "environment":{"type":"local"}
            }
        });

        for tool in TOOLS {
            let input = pre_tool_input_at(tool, tool_input.clone(), &cwd);
            let output = hook_output(&input, "PreToolUse")?;
            assert!(hook_denied(&input)?, "{model}/{thinking}: {input}");
            let reason = output["hookSpecificOutput"]["permissionDecisionReason"]
                .as_str()
                .ok_or("workaround denial reason")?;
            assert!(reason.contains("APP_WORKTREE_REQUIRED"), "{reason}");
            for workaround in [
                "create_worktree",
                "fork_thread",
                "git worktree add",
                "detached directory",
            ] {
                assert!(
                    reason.contains(workaround),
                    "missing {workaround}: {reason}"
                );
            }
        }
    }
    Ok(())
}

pub(super) fn with_project_worktree(mut tool_input: Value) -> Value {
    tool_input["target"] = json!({
        "type":"project",
        "projectId":"local-test-project",
        "environment":{"type":"worktree"}
    });
    tool_input
}
