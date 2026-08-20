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
    generic_or_fallback(policy, capabilities, operation)
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
    // Keep the paired measurement baseline valid, but never let its selected
    // effort override the capability-driven Luna-first generic contract.
    routing_measurement::selected_effort(plugin_root, &corpus, &results)?;
    Ok(generic_or_fallback(policy, capabilities, operation))
}

fn generic_or_fallback(
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
    } else if thread_capabilities::supports(
        capabilities,
        &policy.generic_fallback.model,
        &policy.generic_fallback.thinking,
    ) {
        route(
            "generic",
            operation,
            &policy.generic_fallback.model,
            &policy.generic_fallback.thinking,
        )
    } else {
        json!({"route":policy.fallback})
    }
}

fn route(kind: &str, operation: &str, model: &str, thinking: &str) -> Value {
    json!({"route":kind,"codex_thread_operation":operation,"model":model,"thinking":thinking})
}
