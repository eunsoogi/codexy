use serde_json::{Value, json};

use super::{
    Policy,
    thread_capabilities::{self, RecipientAssignment, ThreadCapabilities},
};
pub(super) fn simple_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
    selected_route(
        policy,
        capabilities,
        operation,
        &policy.simple.model,
        &policy.simple.thinking,
    )
}

pub(super) fn child_to_root_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
    // Missing model or reasoning support selects the policy's root-or-specialist fallback.
    if thread_capabilities::supports(
        capabilities,
        &policy.delivery.child_to_root.model,
        &policy.delivery.child_to_root.thinking,
    ) {
        route(
            "child_to_root",
            operation,
            &policy.delivery.child_to_root.model,
            &policy.delivery.child_to_root.thinking,
        )
    } else {
        json!({"route":policy.fallback})
    }
}

pub(super) fn parent_to_generic_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    assignment: Option<&RecipientAssignment>,
    schema: &str,
    operation: &str,
) -> Value {
    // Role comes from the route direction; a Worker pair may also match the parent pair.
    let Some(assignment) = assignment else {
        let reason = if schema == "codexy.child-routing-request.v1" {
            "Version 1 cannot identify a follow-up recipient; use schema v2 with recipient_assignment."
        } else {
            "Recipient-bound follow-up routing requires recipient_assignment in schema v2; sender settings are not a substitute."
        };
        return json!({"route":policy.fallback,"reason":reason});
    };
    if !policy
        .delivery
        .parent_to_generic
        .iter()
        .any(|route| route.model == assignment.model && route.thinking == assignment.thinking)
    {
        return json!({"route":policy.fallback,"reason":"Assigned recipient model and thinking do not form a supported Worker pair."});
    }
    if !thread_capabilities::supports(capabilities, &assignment.model, &assignment.thinking) {
        return json!({"route":policy.fallback,"reason":"Assigned recipient model and thinking are not supported by declared thread capabilities."});
    }
    route(
        "parent_to_generic",
        operation,
        &assignment.model,
        &assignment.thinking,
    )
}

pub(super) fn selected_general_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
    selected_route(
        policy,
        capabilities,
        operation,
        &policy.general.model,
        &policy.general.thinking,
    )
}

// Difficulty chooses the pair, while capability checks still fail closed on that exact pair.
fn selected_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
    model: &str,
    thinking: &str,
) -> Value {
    if thread_capabilities::supports(capabilities, model, thinking) {
        route("generic", operation, model, thinking)
    } else {
        json!({"route":policy.fallback})
    }
}

fn route(kind: &str, operation: &str, model: &str, thinking: &str) -> Value {
    json!({"route":kind,"codex_thread_operation":operation,"model":model,"thinking":thinking})
}
