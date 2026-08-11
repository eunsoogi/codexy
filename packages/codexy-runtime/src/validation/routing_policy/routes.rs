use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

use super::{
    Policy,
    thread_capabilities::{self, ThreadCapabilities},
};
use crate::validation::routing_measurement;

pub(super) fn simple_route(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Value {
    if thread_capabilities::supports(capabilities, &policy.simple.model, &policy.simple.thinking) {
        route(
            "generic",
            operation,
            &policy.simple.model,
            &policy.simple.thinking,
        )
    } else {
        generic_or_fallback(policy, capabilities, operation, &policy.generic.thinking)
    }
}

pub(super) fn selected_general_route(
    plugin_root: &Path,
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
) -> Result<Value> {
    let corpus = plugin_root.join("skills/orchestration/references/routing-evaluation-corpus.json");
    let results = plugin_root
        .join("skills/orchestration/references")
        .join(&policy.general.measurement_results);
    let corpus = std::fs::read_to_string(corpus)?;
    let results = std::fs::read_to_string(results)?;
    let selected = routing_measurement::selected_effort(plugin_root, &corpus, &results)?;
    Ok(generic_or_fallback(
        policy,
        capabilities,
        operation,
        &selected,
    ))
}

fn generic_or_fallback(
    policy: &Policy,
    capabilities: Option<&ThreadCapabilities>,
    operation: &str,
    thinking: &str,
) -> Value {
    if thread_capabilities::supports(capabilities, &policy.generic.model, thinking) {
        route("generic", operation, &policy.generic.model, thinking)
    } else {
        json!({"route":policy.fallback})
    }
}

fn route(kind: &str, operation: &str, model: &str, thinking: &str) -> Value {
    json!({"route":kind,"codex_thread_operation":operation,"model":model,"thinking":thinking})
}
