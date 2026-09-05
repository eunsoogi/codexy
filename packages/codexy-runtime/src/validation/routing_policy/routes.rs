use serde_json::{Value, json};

use super::{
    Policy,
    thread_capabilities::{self, ThreadCapabilities},
};
pub(super) fn simple_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
    generic_route(policy, capabilities, operation)
}

pub(super) fn child_to_root_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
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
    operation: &str,
) -> Value {
    if thread_capabilities::supports(
        capabilities,
        &policy.delivery.parent_to_generic.model,
        &policy.delivery.parent_to_generic.thinking,
    ) {
        route(
            "parent_to_generic",
            operation,
            &policy.delivery.parent_to_generic.model,
            &policy.delivery.parent_to_generic.thinking,
        )
    } else {
        json!({"route":policy.fallback})
    }
}

pub(super) fn selected_general_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
    generic_route(policy, capabilities, operation)
}

fn generic_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
    if thread_capabilities::supports(
        capabilities,
        &policy.generic.model,
        &policy.generic.thinking,
    ) {
        route(
            "generic",
            operation,
            &policy.generic.model,
            &policy.generic.thinking,
        )
    } else {
        json!({"route":policy.fallback})
    }
}

fn route(kind: &str, operation: &str, model: &str, thinking: &str) -> Value {
    json!({"route":kind,"codex_thread_operation":operation,"model":model,"thinking":thinking})
}
