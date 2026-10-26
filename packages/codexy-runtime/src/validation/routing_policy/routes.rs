use serde::{Deserialize, de::Error as _};
use serde_json::{Value, json};

use super::{
    Policy,
    thread_capabilities::{self, RecipientAssignment, ThreadCapabilities},
};

pub(super) fn deserialize_bounds<'de, D>(
    deserializer: D,
) -> Result<Option<super::BoundedEvidence>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Serde structs also accept positional sequences; only named JSON facts certify these bounds.
    Option::<Value>::deserialize(deserializer)?
        .map(|value| {
            if !value.is_object() {
                return Err(D::Error::custom("bounded_evidence must be a JSON object"));
            }
            serde_json::from_value(value).map_err(D::Error::custom)
        })
        .transpose()
}

pub(super) fn bounded_evidence_is_complete(predicates: Option<&super::Predicates>) -> bool {
    // Human-judged summaries may lack a deterministic oracle; scope, safety and decisions cannot relax.
    predicates.is_some_and(|item| {
        item.fixed_scope
            && item.low_risk_reversible
            && item.no_unresolved_decision
            && item.bounded_evidence.as_ref().is_some_and(|bounds| {
                bounds.read_only
                    && bounds.supplied_inputs_only
                    && (1..=3).contains(&bounds.input_count)
                    && (1..=12_000).contains(&bounds.input_char_count)
                    && (1..=20).contains(&bounds.output_item_limit)
            })
    })
}

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
    hard_work: bool,
) -> Value {
    // Ordinary implementation is the default Worker route; only evidenced hard work may escalate.
    if hard_work {
        selected_route(
            policy,
            capabilities,
            operation,
            &policy.hard_work.model,
            &policy.hard_work.thinking,
        )
    } else {
        simple_route(policy, capabilities, operation)
    }
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
