//! Verifies that Worker creation denials carry corrective guidance in the model context.

use super::*;

#[test]
fn pre_tool_denial_delivers_actionable_model_context_without_echoing_the_request() -> TestResult {
    let tool_input = worktree_environment::with_project_worktree(json!({
        "prompt":"PRIVATE_PROMPT_SENTINEL",
    }));
    let input = pre_tool_input(TOOLS[0], tool_input);
    let output = hook_output(&input, "PreToolUse")?;
    let specific = &output["hookSpecificOutput"];

    assert_eq!(specific["permissionDecision"], "deny");
    let reason = specific["permissionDecisionReason"]
        .as_str()
        .ok_or("denial reason")?;
    assert!(reason.contains("MISSING_ROUTE_FIELDS"), "{reason}");
    let context = specific["additionalContext"]
        .as_str()
        .ok_or("model additionalContext")?;
    for requirement in [
        "MISSING_ROUTE_FIELDS",
        "model and thinking",
        "reuse any already-selected Worker pair",
        "if none is selected, classify the task evidence",
        "gpt-6-luna",
        "gpt-6.1-terra",
        "projectId from the saved project selected for this task",
        "target.environment.type='worktree'",
        "correct only the identified fields in the original create_thread request",
        "prevented before create_thread executed",
        "outcome is unknown",
        "inspect that exact result to distinguish a newly identified input error",
        "Correct another field only when the result identifies it",
        "MUST NOT try another model or worktree route",
        "repeat an unchanged request",
        "bypass host policy",
    ] {
        assert!(context.contains(requirement), "missing {requirement}: {context}");
    }
    assert!(!context.contains("PRIVATE_PROMPT_SENTINEL"), "{context}");
    assert!(!context.contains("local-test-project"), "{context}");
    Ok(())
}

#[test]
fn app_worktree_denial_explains_the_value_source_without_inventing_an_id() -> TestResult {
    let input = pre_tool_input(
        TOOLS[0],
        json!({
            "model":"gpt-6-luna",
            "thinking":"max",
            "target":{
                "type":"project",
                "projectId":"local-test-project",
                "environment":{"type":"local"}
            }
        }),
    );
    let output = hook_output(&input, "PreToolUse")?;
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("worktree model additionalContext")?;

    assert!(context.contains("APP_WORKTREE_REQUIRED"), "{context}");
    assert!(context.contains("projectId from the saved project selected for this task"), "{context}");
    assert!(context.contains("target.type='project'"), "{context}");
    assert!(context.contains("target.environment.type='worktree'"), "{context}");
    assert!(context.contains("MUST NOT invent a projectId"), "{context}");
    assert!(!context.contains("local-test-project"), "{context}");
    Ok(())
}

#[test]
fn route_field_feedback_is_error_specific_and_preserves_existing_selection() -> TestResult {
    let cases = [
        (
            "missing model",
            json!({"thinking":"max","prompt":"PRIVATE_PROMPT_SENTINEL"}),
            "MISSING_MODEL",
            "if none is selected, classify the task evidence",
        ),
        (
            "missing Luna reasoning",
            json!({"model":"gpt-6-luna","prompt":"PRIVATE_PROMPT_SENTINEL"}),
            "MISSING_THINKING",
            "preserve model='gpt-6-luna' and set thinking='max'",
        ),
        (
            "crossed hard pair",
            json!({"model":"gpt-6.1-terra","thinking":"max"}),
            "UNSUPPORTED_THINKING",
            "preserve model='gpt-6.1-terra' and set thinking='medium'",
        ),
        (
            "unsupported model",
            json!({"model":"untrusted-model-value","thinking":"medium"}),
            "UNSUPPORTED_MODEL",
            "assignment and its evidence",
        ),
    ];

    for (label, route, code, expected_guidance) in cases {
        let input = pre_tool_input(TOOLS[0], worktree_environment::with_project_worktree(route));
        let output = hook_output(&input, "PreToolUse")?;
        let context = output["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .ok_or("route model additionalContext")?;
        assert!(context.contains(code), "{label}: {context}");
        assert!(context.contains(expected_guidance), "{label}: {context}");
        assert!(context.contains("MUST correct only the identified fields"), "{label}: {context}");
        assert!(context.contains("MUST NOT ask the user to select or reconfirm"), "{label}: {context}");
        assert!(context.contains("host permission or capability restriction"), "{label}: {context}");
        assert!(!context.contains("PRIVATE_PROMPT_SENTINEL"), "{label}: {context}");
        assert!(!context.contains("untrusted-model-value"), "{label}: {context}");
    }
    Ok(())
}

#[test]
fn parent_checkout_errors_are_distinguished_from_request_field_errors() -> TestResult {
    let (_temp, _primary, linked) = git_worktree_fixture()?;
    let input = pre_tool_input_at(
        TOOLS[0],
        worktree_environment::with_project_worktree(json!({
            "model":"gpt-6-luna",
            "thinking":"max"
        })),
        &linked,
    );
    let output = hook_output(&input, "PreToolUse")?;
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("parent-worktree model additionalContext")?;

    assert!(context.contains("PARENT_WORKTREE"), "{context}");
    assert!(context.contains("parent-checkout constraint, not an incorrect create_thread argument"), "{context}");
    assert!(context.contains("different controlling task"), "{context}");
    assert!(context.contains("MUST NOT change request fields"), "{context}");
    assert!(!context.contains("MUST correct only the identified fields"), "{context}");
    Ok(())
}

#[test]
fn unverified_parent_checkout_reports_the_missing_execution_context() -> TestResult {
    let temp = tempfile::tempdir()?;
    let missing_cwd = temp.path().join("missing-parent-checkout");
    let input = pre_tool_input_at(
        TOOLS[0],
        worktree_environment::with_project_worktree(json!({
            "model":"gpt-6-luna",
            "thinking":"max"
        })),
        &missing_cwd,
    );
    let output = hook_output(&input, "PreToolUse")?;
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("unverified-parent model additionalContext")?;

    assert!(context.contains("PARENT_CHECKOUT_UNVERIFIED"), "{context}");
    assert!(context.contains("different controlling task"), "{context}");
    assert!(context.contains("outcome is unknown"), "{context}");
    assert!(context.contains("MUST NOT change request fields"), "{context}");
    assert!(!context.contains("MUST correct only the identified fields"), "{context}");
    Ok(())
}

#[test]
fn permission_request_keeps_corrective_guidance_in_the_denial_message() -> TestResult {
    let mut input = pre_tool_input(
        TOOLS[0],
        worktree_environment::with_project_worktree(json!({"thinking":"max"})),
    );
    input["hook_event_name"] = json!("PermissionRequest");
    let output = hook_output(&input, "PermissionRequest")?;
    let specific = &output["hookSpecificOutput"];
    let message = specific["decision"]["message"]
        .as_str()
        .ok_or("permission denial message")?;

    assert_eq!(specific["decision"]["behavior"], "deny");
    assert!(message.contains("MISSING_MODEL"), "{message}");
    assert!(message.contains("correct only the identified fields"), "{message}");
    assert!(message.contains("prevented before create_thread executed"), "{message}");
    assert!(specific.get("additionalContext").is_none());
    Ok(())
}

#[test]
fn incomplete_hard_selection_supplies_its_matching_setting_without_downgrading() -> TestResult {
    let input = pre_tool_input(
        TOOLS[0],
        worktree_environment::with_project_worktree(json!({"model":"gpt-6.1-terra"})),
    );
    let output = hook_output(&input, "PreToolUse")?;
    let context = output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("hard route model additionalContext")?;

    assert!(
        context.contains("preserve model='gpt-6.1-terra' and set thinking='medium'"),
        "{context}"
    );
    assert!(context.contains("only when the task evidence meets the hard-work criteria"));

    let invalid_pair = pre_tool_input(
        TOOLS[0],
        worktree_environment::with_project_worktree(
            json!({"model":"gpt-6.1-terra","thinking":"max"}),
        ),
    );
    let invalid_output = hook_output(&invalid_pair, "PreToolUse")?;
    let invalid_context = invalid_output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("invalid hard route model additionalContext")?;
    assert!(
        invalid_context.contains("preserve model='gpt-6.1-terra' and set thinking='medium'"),
        "{invalid_context}"
    );
    Ok(())
}
