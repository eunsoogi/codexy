use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

use super::routing_json;

mod routes;
mod thread_capabilities;
use routes::{
    bounded_evidence_is_complete, child_to_root_route, parent_to_generic_route,
    selected_general_route, simple_route,
};
use thread_capabilities::{ThreadDirection, simple_is_complete, worker_scope_is_complete};

// The schema string, policy field names, and route labels are serialized
// compatibility identifiers; product-role terminology is standardized in
// diagnostics and model constants below.
const SIMPLE_WORKER_MODEL: &str = "gpt-6-luna";
const SIMPLE_WORKER_THINKING: &str = "max";
const HARD_WORKER_MODEL: &str = "gpt-6.1-sol";
const HARD_WORKER_THINKING: &str = "medium";
const ASSIGNED_WORKER_ROUTES: [(&str, &str); 2] = [
    (SIMPLE_WORKER_MODEL, SIMPLE_WORKER_THINKING),
    (HARD_WORKER_MODEL, HARD_WORKER_THINKING),
];
// Parent requests use Sol/Light; Worker selection and authenticated directions stay distinct.
const ORCHESTRATOR_MODEL: &str = "gpt-6.1-sol";
const ORCHESTRATOR_THINKING: &str = "low";

pub(super) struct Policy {
    pub(super) schema: String,
    pub(super) hard_work: Route,
    pub(super) named_specialist: Specialist,
    pub(super) simple: Simple,
    pub(super) fallback: String,
    pub(super) delivery: Delivery,
}

pub(super) struct Route {
    pub(super) model: String,
    pub(super) thinking: String,
}

pub(super) struct Specialist {
    pub(super) catalog: String,
    pub(super) caller_overrides: String,
}

pub(super) struct Simple {
    pub(super) model: String,
    pub(super) thinking: String,
    pub(super) all_required: Vec<String>,
}

pub(super) struct Delivery {
    pub(super) parent_to_generic: Vec<Route>,
    pub(super) child_to_root: Route,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Predicates {
    fixed_scope: bool,
    deterministic_oracle: bool,
    low_risk_reversible: bool,
    no_unresolved_decision: bool,
    // Optional evidence bounds extend the legacy container without changing old simple requests.
    #[serde(default, deserialize_with = "routes::deserialize_bounds")]
    bounded_evidence: Option<BoundedEvidence>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundedEvidence {
    read_only: bool,
    supplied_inputs_only: bool,
    input_count: u8,
    input_char_count: u16,
    output_item_limit: u8,
}

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let _ = plugin_root;
    validate(&contract())
        .err()
        .into_iter()
        .map(|error| error.to_string())
        .collect()
}

pub(super) fn resolve(plugin_root: &Path, request: &str) -> Result<Value> {
    // Recipient-bound follow-ups are checked first; named specialists precede classification-based routes.
    let policy = contract();
    let request = thread_capabilities::parse_request(request)?;
    // A follow-up targets an already assigned Worker, so classification or specialist fields cannot bypass recipient checks.
    if request.codex_thread_direction == Some(ThreadDirection::OrchestratorToWorker) {
        return Ok(parent_to_generic_route(
            &policy,
            request.codex_thread_capabilities.as_ref(),
            request.recipient_assignment.as_ref(),
            &request.schema,
            &request.codex_thread_operation,
        ));
    }
    if let Some(agent_type) = request
        .named_specialist
        .filter(|value| !value.trim().is_empty())
    {
        if !known_specialist(plugin_root, &policy, &agent_type)? {
            bail!("Worker routing request names an unknown packaged specialist");
        }
        return Ok(json!({"route":"named_specialist","agent_type":agent_type}));
    }
    if request.codex_thread_direction == Some(ThreadDirection::WorkerToOrchestrator) {
        return Ok(child_to_root_route(
            &policy,
            request.codex_thread_capabilities.as_ref(),
            &request.codex_thread_operation,
        ));
    }
    match request.classification.as_str() {
        "bounded_evidence" if bounded_evidence_is_complete(request.simple_predicates.as_ref()) => {
            Ok(simple_route(
                &policy,
                request.codex_thread_capabilities.as_ref(),
                &request.codex_thread_operation,
            ))
        }
        "simple" if simple_is_complete(request.simple_predicates.as_ref()) => Ok(simple_route(
            &policy,
            request.codex_thread_capabilities.as_ref(),
            &request.codex_thread_operation,
        )),
        "general" if worker_scope_is_complete(request.simple_predicates.as_ref()) => {
            Ok(selected_general_route(
                &policy,
                request.codex_thread_capabilities.as_ref(),
                &request.codex_thread_operation,
                request.hard_work.is_some(),
            ))
        }
        "general" | "simple" | "bounded_evidence" | "ambiguous" | "high_risk" | "incomplete" => {
            Ok(json!({"route":policy.fallback}))
        }
        _ => bail!("Worker routing request classification is not recognized"),
    }
}

fn known_specialist(plugin_root: &Path, policy: &Policy, agent_type: &str) -> Result<bool> {
    let catalog = std::fs::read_to_string(plugin_root.join(&policy.named_specialist.catalog))?;
    let catalog: toml::Value = toml::from_str(&catalog)?;
    let expected = format!("{agent_type}.toml");
    Ok(catalog
        .get("agent_files")
        .and_then(toml::Value::as_array)
        .is_some_and(|files| files.iter().any(|file| file.as_str() == Some(&expected))))
}

fn contract() -> Policy {
    Policy {
        schema: "codexy.child-routing-policy.v1".to_owned(),
        hard_work: Route {
            model: HARD_WORKER_MODEL.to_owned(),
            thinking: HARD_WORKER_THINKING.to_owned(),
        },
        named_specialist: Specialist {
            catalog: "agents/catalog.toml".to_owned(),
            caller_overrides: "forbidden".to_owned(),
        },
        simple: Simple {
            model: SIMPLE_WORKER_MODEL.to_owned(),
            thinking: SIMPLE_WORKER_THINKING.to_owned(),
            all_required: [
                "fixed_scope",
                "deterministic_oracle",
                "low_risk_reversible",
                "no_unresolved_decision",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        },
        fallback: "root_or_named_specialist".to_owned(),
        delivery: Delivery {
            // Keep this allowlist aligned with the authenticated thread-delivery hook.
            parent_to_generic: ASSIGNED_WORKER_ROUTES
                .iter()
                .map(|(model, thinking)| Route {
                    model: (*model).to_owned(),
                    thinking: (*thinking).to_owned(),
                })
                .collect(),
            child_to_root: Route {
                model: ORCHESTRATOR_MODEL.to_owned(),
                thinking: ORCHESTRATOR_THINKING.to_owned(),
            },
        },
    }
}

fn validate(policy: &Policy) -> Result<()> {
    let required = [
        "fixed_scope",
        "deterministic_oracle",
        "low_risk_reversible",
        "no_unresolved_decision",
    ];
    if policy.schema != "codexy.child-routing-policy.v1"
        || policy.hard_work.model != HARD_WORKER_MODEL
        || policy.hard_work.thinking != HARD_WORKER_THINKING
        || policy.named_specialist.catalog != "agents/catalog.toml"
        || policy.named_specialist.caller_overrides != "forbidden"
        || policy.simple.model != SIMPLE_WORKER_MODEL
        || policy.simple.thinking != SIMPLE_WORKER_THINKING
        || !policy
            .simple
            .all_required
            .iter()
            .map(String::as_str)
            .eq(required)
        || policy.fallback != "root_or_named_specialist"
        || !policy
            .delivery
            .parent_to_generic
            .iter()
            .map(|route| (route.model.as_str(), route.thinking.as_str()))
            .eq(ASSIGNED_WORKER_ROUTES)
        || policy.delivery.child_to_root.model != ORCHESTRATOR_MODEL
        || policy.delivery.child_to_root.thinking != ORCHESTRATOR_THINKING
    {
        bail!(
            "Worker routing policy must retain the closed named-specialist-first fail-closed contract"
        );
    }
    Ok(())
}
