//! Verifies that routing selection carries the app target into creation input.

use super::{DEFAULT_WORKER, check, request};
use crate::support::TestResult;
use serde_json::{Value, json};

#[test]
fn route_builder_preserves_app_worktree_target_in_create_thread_input() -> TestResult {
    let mut assignment = request("general", DEFAULT_WORKER);
    assignment["creation_prompt"] = json!("Resolve the assigned issue within its verified scope.");
    assignment["creation_target"] = json!({
        "type":"project",
        "projectId":"assignment-selected-project",
        "environment":{"type":"worktree"}
    });

    check(
        assignment,
        json!({
            "route":"generic",
            "codex_thread_operation":"create_thread",
            "model":DEFAULT_WORKER.0,
            "thinking":DEFAULT_WORKER.1,
            "tool_input":{
                "prompt":"Resolve the assigned issue within its verified scope.",
                "model":DEFAULT_WORKER.0,
                "thinking":DEFAULT_WORKER.1,
                "target":{
                    "type":"project",
                    "projectId":"assignment-selected-project",
                    "environment":{"type":"worktree"}
                }
            }
        }),
    )?;
    Ok(())
}

#[test]
fn incomplete_prompt_or_non_worktree_target_fails_closed() -> TestResult {
    let mut missing_prompt = request("general", DEFAULT_WORKER);
    missing_prompt["creation_prompt"] = json!("  ");
    check(
        missing_prompt,
        json!({
            "route":"root_or_named_specialist",
            "reason":"Worker creation requires a non-empty creation_prompt."
        }),
    )?;

    let mut absent_prompt = request("general", DEFAULT_WORKER);
    absent_prompt
        .as_object_mut()
        .ok_or("request object")?
        .remove("creation_prompt");
    check(
        absent_prompt,
        json!({
            "route":"root_or_named_specialist",
            "reason":"Worker creation requires a non-empty creation_prompt."
        }),
    )?;

    for target in [
        Value::Null,
        json!({"type":"projectless"}),
        json!({
            "type":"project",
            "projectId":"saved-codexy-project",
            "environment":{"type":"local"}
        }),
        json!({
            "type":"project",
            "projectId":"  ",
            "environment":{"type":"worktree"}
        }),
        json!({
            "type":"project",
            "projectId":" saved-codexy-project ",
            "environment":{"type":"worktree"}
        }),
    ] {
        let mut invalid_target = request("general", DEFAULT_WORKER);
        if target.is_null() {
            invalid_target
                .as_object_mut()
                .ok_or("request object")?
                .remove("creation_target");
        } else {
            invalid_target["creation_target"] = target;
        }
        check(
            invalid_target,
            json!({
                "route":"root_or_named_specialist",
                "reason":"Worker creation requires a project target with a non-empty projectId without surrounding whitespace and a worktree environment."
            }),
        )?;
    }
    Ok(())
}
