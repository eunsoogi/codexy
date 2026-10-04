//! Requires the original Worker creation request to name a project worktree;
//! local and incomplete targets cannot be retrofitted later.

use super::*;

#[test]
fn app_project_worktree_is_admitted_for_the_assigned_worker_pair() -> TestResult {
    let cwd = primary_checkout();
    let tool_input = json!({
        "model":"gpt-6-luna",
        "thinking":"max",
        "target":{
            "type":"project",
            "projectId":"local-test-project",
            "environment":{"type":"worktree"}
        }
    });

    for tool in TOOLS {
        for event in ["PreToolUse", "PermissionRequest"] {
            let mut input = pre_tool_input_at(tool, tool_input.clone(), &cwd);
            input["hook_event_name"] = json!(event);
            assert!(!hook_denied_for(&input, event)?, "{event}: {input}");
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
        (
            "projectless task",
            json!({"target":{"type":"projectless"}}),
        ),
        (
            "missing project identity",
            json!({"target":{"type":"project","environment":{"type":"worktree"}}}),
        ),
        (
            "worktree without project target",
            json!({"target":{"environment":{"type":"worktree"}}}),
        ),
    ];

    for (label, target) in cases {
        let mut tool_input = json!({"model":"gpt-6-luna","thinking":"max"});
        tool_input.as_object_mut().ok_or("tool input object")?.extend(
            target.as_object().ok_or("target object")?.clone(),
        );
        for tool in TOOLS {
            for event in ["PreToolUse", "PermissionRequest"] {
                let mut input = pre_tool_input_at(tool, tool_input.clone(), &cwd);
                input["hook_event_name"] = json!(event);
                let output = hook_output(&input, event)?;
                assert!(hook_denied_for(&input, event)?, "{label}: {event}: {input}");
                let reason = if event == "PermissionRequest" {
                    output["hookSpecificOutput"]["decision"]["message"].as_str()
                } else {
                    output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
                }
                .ok_or("worktree admission denial reason")?;
                assert!(reason.contains("APP_WORKTREE_REQUIRED"), "{label}: {reason}");
                assert!(reason.contains("target.environment.type"), "{label}: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn a_local_child_cannot_be_admitted_by_promising_a_later_worktree() -> TestResult {
    let cwd = primary_checkout();
    let tool_input = json!({
        "model":"gpt-6-luna",
        "thinking":"max",
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
        assert!(hook_denied(&input)?, "{input}");
        let reason = output["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .ok_or("workaround denial reason")?;
        assert!(reason.contains("APP_WORKTREE_REQUIRED"), "{reason}");
        for workaround in ["create_worktree", "fork_thread", "git worktree add", "detached directory"] {
            assert!(reason.contains(workaround), "missing {workaround}: {reason}");
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
