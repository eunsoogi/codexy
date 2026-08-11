use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

use super::{routing_json, routing_measurement};

mod capabilities;
use capabilities::Capabilities;

const POLICY_PATH: &str = "skills/orchestration/references/child-routing-policy.json";
const REQUEST_SCHEMA: &str = "codexy.child-routing-request.v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema: String,
    generic: Route,
    named_specialist: Specialist,
    simple: Simple,
    general: General,
    fallback: String,
    delivery: Delivery,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Route {
    model: String,
    thinking: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Specialist {
    catalog: String,
    caller_overrides: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Simple {
    model: String,
    thinking: String,
    all_required: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct General {
    model: String,
    candidate_efforts: Vec<String>,
    measurement_results: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delivery {
    parent_to_generic: Route,
    child_to_root: Route,
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
    #[serde(default)]
    recipient_capabilities: Option<Capabilities>,
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
            let errors = routing_measurement::check_canonical(plugin_root, &results);
            errors
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
            request.recipient_capabilities.as_ref(),
        )),
        "general" => selected_general_route(
            plugin_root,
            &policy,
            request.recipient_capabilities.as_ref(),
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
        || policy.generic.model != "gpt-5.6-terra"
        || policy.generic.thinking != "high"
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
        || policy.general.model != "gpt-5.6-terra"
        || !policy
            .general
            .candidate_efforts
            .iter()
            .map(String::as_str)
            .eq(["high", "xhigh", "max"])
        || policy.general.measurement_results != "routing-evaluation-results.json"
        || policy.fallback != "root_or_named_specialist"
        || policy.delivery.parent_to_generic.model != policy.generic.model
        || policy.delivery.parent_to_generic.thinking != policy.generic.thinking
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

fn simple_route(policy: &Policy, capabilities: Option<&Capabilities>) -> Value {
    if capabilities::supports(capabilities, &policy.simple.model, &policy.simple.thinking) {
        route("generic", &policy.simple.model, &policy.simple.thinking)
    } else {
        generic_or_fallback(policy, capabilities, &policy.generic.thinking)
    }
}

fn selected_general_route(
    plugin_root: &Path,
    policy: &Policy,
    capabilities: Option<&Capabilities>,
) -> Result<Value> {
    let corpus = plugin_root.join("skills/orchestration/references/routing-evaluation-corpus.json");
    let results = plugin_root
        .join("skills/orchestration/references")
        .join(&policy.general.measurement_results);
    let corpus = std::fs::read_to_string(corpus)?;
    let results = std::fs::read_to_string(results)?;
    let selected = routing_measurement::selected_effort(plugin_root, &corpus, &results)?;
    Ok(generic_or_fallback(policy, capabilities, &selected))
}

fn generic_or_fallback(
    policy: &Policy,
    capabilities: Option<&Capabilities>,
    thinking: &str,
) -> Value {
    if capabilities::supports(capabilities, &policy.generic.model, thinking) {
        route("generic", &policy.generic.model, thinking)
    } else {
        json!({"route":policy.fallback})
    }
}

fn route(kind: &str, model: &str, thinking: &str) -> Value {
    json!({"route":kind,"model":model,"thinking":thinking})
}
