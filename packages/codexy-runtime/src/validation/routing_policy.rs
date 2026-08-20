use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

use super::{routing_json, routing_measurement};

mod routes;
mod thread_capabilities;
use routes::{selected_general_route, simple_route};
use thread_capabilities::ThreadCapabilities;

const POLICY_PATH: &str = "skills/orchestration/references/child-routing-policy.json";
const REQUEST_SCHEMA: &str = "codexy.child-routing-request.v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Policy {
    pub(super) schema: String,
    pub(super) generic: Route,
    pub(super) generic_fallback: Route,
    pub(super) named_specialist: Specialist,
    pub(super) simple: Simple,
    pub(super) general: General,
    pub(super) fallback: String,
    pub(super) delivery: Delivery,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Route {
    pub(super) model: String,
    pub(super) thinking: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Specialist {
    pub(super) catalog: String,
    pub(super) caller_overrides: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Simple {
    pub(super) model: String,
    pub(super) thinking: String,
    pub(super) all_required: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct General {
    pub(super) candidate_efforts: Vec<String>,
    pub(super) measurement_results: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Delivery {
    pub(super) parent_to_generic: GenericDelivery,
    pub(super) child_to_root: Route,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GenericDelivery {
    pub(super) primary: Route,
    pub(super) fallback: Route,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    classification: String,
    #[serde(default)]
    named_specialist: Option<String>,
    #[serde(default)]
    simple_predicates: Option<Predicates>,
    codex_thread_operation: String,
    #[serde(default)]
    codex_thread_capabilities: Option<ThreadCapabilities>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Predicates {
    fixed_scope: bool,
    deterministic_oracle: bool,
    low_risk_reversible: bool,
    no_unresolved_decision: bool,
}

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    load(plugin_root).map_or_else(
        |error| vec![error.to_string()],
        |policy| {
            let results = plugin_root
                .join("skills/orchestration/references")
                .join(&policy.general.measurement_results);

            routing_measurement::check_canonical(plugin_root, &results)
        },
    )
}

pub(super) fn resolve(plugin_root: &Path, request: &str) -> Result<Value> {
    let policy = load(plugin_root)?;
    let request = parse_request(request)?;
    if let Some(agent_type) = request
        .named_specialist
        .filter(|value| !value.trim().is_empty())
    {
        if !known_specialist(plugin_root, &policy, &agent_type)? {
            bail!("child routing request names an unknown packaged specialist");
        }
        return Ok(json!({"route":"named_specialist","agent_type":agent_type}));
    }
    match request.classification.as_str() {
        "simple" if simple_is_complete(request.simple_predicates.as_ref()) => Ok(simple_route(
            &policy,
            request.codex_thread_capabilities.as_ref(),
            &request.codex_thread_operation,
        )),
        "general" => selected_general_route(
            plugin_root,
            &policy,
            request.codex_thread_capabilities.as_ref(),
            &request.codex_thread_operation,
        ),
        "simple" | "ambiguous" | "high_risk" | "incomplete" => Ok(json!({"route":policy.fallback})),
        _ => bail!("child routing request classification is not recognized"),
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

fn load(plugin_root: &Path) -> Result<Policy> {
    let path = plugin_root.join(POLICY_PATH);
    let text = std::fs::read_to_string(&path)?;
    let value = routing_json::parse(&text).map_err(anyhow::Error::msg)?;
    let policy = serde_json::from_value::<Policy>(value)?;
    validate(&policy)?;
    Ok(policy)
}

fn parse_request(text: &str) -> Result<Request> {
    let value = routing_json::parse(text).map_err(anyhow::Error::msg)?;
    let request = serde_json::from_value::<Request>(value)?;
    if request.schema != REQUEST_SCHEMA {
        bail!("child routing request has an unsupported schema");
    }
    thread_capabilities::validate_operation(&request.codex_thread_operation)?;
    Ok(request)
}

fn validate(policy: &Policy) -> Result<()> {
    let required = [
        "fixed_scope",
        "deterministic_oracle",
        "low_risk_reversible",
        "no_unresolved_decision",
    ];
    if policy.schema != "codexy.child-routing-policy.v1"
        || policy.generic.model != "gpt-5.6-luna"
        || policy.generic.thinking != "max"
        || policy.generic_fallback.model != "gpt-5.6-terra"
        || policy.generic_fallback.thinking != "high"
        || policy.named_specialist.catalog != "agents/catalog.toml"
        || policy.named_specialist.caller_overrides != "forbidden"
        || policy.simple.model != "gpt-5.6-luna"
        || policy.simple.thinking != "max"
        || !policy
            .simple
            .all_required
            .iter()
            .map(String::as_str)
            .eq(required)
        || !policy
            .general
            .candidate_efforts
            .iter()
            .map(String::as_str)
            .eq(["high", "xhigh", "max"])
        || policy.general.measurement_results != "routing-evaluation-results.json"
        || policy.fallback != "root_or_named_specialist"
        || policy.delivery.parent_to_generic.primary.model != policy.generic.model
        || policy.delivery.parent_to_generic.primary.thinking != policy.generic.thinking
        || policy.delivery.parent_to_generic.fallback.model != policy.generic_fallback.model
        || policy.delivery.parent_to_generic.fallback.thinking != policy.generic_fallback.thinking
        || policy.delivery.child_to_root.model != "gpt-5.6-sol"
        || policy.delivery.child_to_root.thinking != "medium"
    {
        bail!(
            "child routing policy must retain the closed named-specialist-first fail-closed contract"
        );
    }
    Ok(())
}

fn simple_is_complete(predicates: Option<&Predicates>) -> bool {
    predicates.is_some_and(|item| {
        item.fixed_scope
            && item.deterministic_oracle
            && item.low_risk_reversible
            && item.no_unresolved_decision
    })
}
