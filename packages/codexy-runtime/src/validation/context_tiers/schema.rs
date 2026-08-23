use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::super::{BudgetSemantics, FORWARDED_CONTEXT_TYPES, ForwardedContext};

#[rustfmt::skip]
const SAFETY_FIELDS: [&str; 10] = ["issue_pr_identity", "owner_worktree", "base_head_sha", "dirty_index_state", "checks", "unresolved_review_threads", "selected_reviewer_state", "verification", "external_gate", "next_action"];
#[rustfmt::skip]
const TASK_CLASSES: [&str; 10] = ["orchestration/lane setup", "implementation", "review response", "GitHub/merge", "validation/QA", "documentation/skill authoring", "plugin/release", "investigation/debugging", "issue/intake only", "other"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Contract {
    pub schema: String,
    pub tier_order: Vec<String>,
    pub identity_order: Vec<String>,
    pub authorities: Vec<Authority>,
    pub retained_fields: Vec<RetainedField>,
    pub profile_matrix: BTreeMap<String, BTreeMap<String, String>>,
    pub omission_reasons: Vec<String>,
    pub routing: Routing,
    pub ordering: Ordering,
    pub budget_semantics: BudgetSemantics,
    pub forwarded_context_types: Vec<String>,
    pub forbidden_context: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Authority {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RetainedField {
    pub name: String,
    pub tier: String,
    pub identity: String,
    pub authority: String,
    pub safety_invariant: bool,
    pub budget_exempt: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Routing {
    pub task_classes: Vec<String>,
    pub task_reference_routes: BTreeMap<String, Vec<String>>,
    pub fail_closed_classes: Vec<String>,
    pub fallback_authority: String,
    pub fail_closed_preserves_all_safety_fields: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ordering {
    pub stable_prefix: String,
    pub volatile_prefix: String,
    pub stable_identity: String,
    pub volatile_identity: String,
    pub stable_fields: Vec<String>,
    pub volatile_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope {
    pub schema: String,
    pub profile: String,
    pub task_class: String,
    pub route_authority: Option<String>,
    pub action_allowed: bool,
    pub slots: BTreeMap<String, Slot>,
    #[serde(rename = "forwarded_context")]
    pub _forwarded_context: Vec<ForwardedContext>,
    pub stable_identity: String,
    pub volatile_identity: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CurrentState {
    pub schema: String,
    pub slots: BTreeMap<String, Slot>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub(super) enum Slot {
    Present(Present),
    Omitted(Omitted),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Present {
    pub value: Value,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Omitted {
    pub omitted: Omission,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Omission {
    pub code: String,
    pub reason: String,
}

pub(super) fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    let value = super::super::routing_json::parse(text).map_err(anyhow::Error::msg)?;
    serde_json::from_value(value).map_err(anyhow::Error::from)
}

pub(super) fn digest(prefix: &str, bytes: &[u8]) -> String {
    format!("{prefix}:{:x}", Sha256::digest(bytes))
}

pub(super) fn slot_string<'a>(slots: &'a BTreeMap<String, Slot>, name: &str) -> Option<&'a str> {
    match slots.get(name) {
        Some(Slot::Present(present)) => present.value.as_str(),
        _ => None,
    }
}

pub(super) fn validate(contract: &Contract, plugin_root: &std::path::Path) -> Result<()> {
    if contract.schema != "codexy.context-tiers.v1"
        || contract.tier_order != ["always_on", "task_selected", "event_delta", "refresh_only"]
        || contract.identity_order != ["stable", "volatile"]
    {
        bail!("context contract has unknown tiers or identity order");
    }
    let authority_ids = contract
        .authorities
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let authority_paths = contract
        .authorities
        .iter()
        .map(|item| item.path.as_str())
        .collect::<BTreeSet<_>>();
    if authority_ids.len() != contract.authorities.len()
        || authority_paths.len() != contract.authorities.len()
        || contract
            .authorities
            .iter()
            .any(|item| !plugin_root.join(&item.path).is_file())
    {
        bail!("context authorities must be unique and resolve inside the plugin");
    }
    let field_names = contract
        .retained_fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();
    if field_names.len() != 15
        || field_names.len() != contract.retained_fields.len()
        || contract.retained_fields.iter().any(|field| {
            !contract.tier_order.contains(&field.tier)
                || !contract.identity_order.contains(&field.identity)
                || !authority_ids.contains(field.authority.as_str())
        })
    {
        bail!("retained fields must have one closed tier, identity, and authority");
    }
    if SAFETY_FIELDS.iter().any(|required| {
        !contract.retained_fields.iter().any(|field| {
            field.name == *required
                && field.tier == "always_on"
                && field.identity == "volatile"
                && field.safety_invariant
                && field.budget_exempt
        })
    }) {
        bail!("safety fields must remain always-on and budget-exempt");
    }
    if contract.profile_matrix.len() != 3
        || ["light", "standard", "strict"].iter().any(|profile| {
            contract.profile_matrix.get(*profile).is_none_or(|tiers| {
                tiers.len() != 4
                    || contract
                        .tier_order
                        .iter()
                        .any(|tier| !tiers.contains_key(tier))
                    || tiers["always_on"] != "required"
            })
        })
    {
        bail!("profiles must decide every tier and retain always-on context");
    }
    if contract.routing.task_classes != TASK_CLASSES
        || contract.routing.task_reference_routes.len() != contract.routing.task_classes.len()
        || contract
            .routing
            .task_classes
            .iter()
            .any(|task| !contract.routing.task_reference_routes.contains_key(task))
        || contract
            .routing
            .task_reference_routes
            .values()
            .flatten()
            .any(|authority| !authority_ids.contains(authority.as_str()))
        || contract.routing.fallback_authority != "child_routing"
        || !contract.routing.fail_closed_preserves_all_safety_fields
    {
        bail!("task references and risk routes must remain closed and authority-backed");
    }
    let ordered = contract
        .ordering
        .stable_fields
        .iter()
        .chain(&contract.ordering.volatile_fields)
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if ordered != field_names
        || contract.ordering.stable_prefix == contract.ordering.volatile_prefix
        || contract.ordering.stable_identity
            != "sha256_exact_contract_bytes_and_typed_stable_fields"
        || contract.ordering.volatile_identity != "sha256_typed_envelope_in_field_order"
        || contract.budget_semantics.context_unit != "utf8_bytes_after_serialization"
        || contract.budget_semantics.output_unit != "utf8_bytes_emitted"
        || contract.budget_semantics.limit_source != "positive_consumer_supplied_per_stage"
        || contract.budget_semantics.context_floor != ["always_on", "applicable_task_selected"]
        || contract.budget_semantics.cache_metadata != "unavailable_not_zero"
        || contract.budget_semantics.cache_savings_claims != "prohibited_without_runtime_evidence"
        || contract.forwarded_context_types != FORWARDED_CONTEXT_TYPES
        || contract.forbidden_context
            != [
                "full_conversation_forwarding",
                "full_tool_body_forwarding",
                "full_agent_tree_forwarding",
            ]
    {
        bail!("context ordering, budget, omission, or forbidden-content semantics were weakened");
    }
    Ok(())
}
