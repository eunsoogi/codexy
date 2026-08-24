use std::collections::BTreeMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::super::{BudgetSemantics, ForwardedContext};

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
    pub surface_names: Vec<String>,
    pub surface_reference_routes: BTreeMap<String, Vec<String>>,
    pub surface_non_applicable_fields: BTreeMap<String, Vec<String>>,
    pub risk_names: Vec<String>,
    pub risk_reference_routes: BTreeMap<String, Vec<String>>,
    pub fallback_reference_route: Vec<String>,
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

#[derive(Debug, Clone)]
pub(super) struct Classification {
    pub workflow: String,
    pub surfaces: Vec<String>,
    pub risks: Vec<String>,
}

pub(super) fn classification(slot: &Slot, contract: &Contract) -> Option<Classification> {
    let Slot::Present(present) = slot else {
        return None;
    };
    match &present.value {
        Value::String(workflow) => Some(Classification {
            workflow: workflow.clone(),
            surfaces: vec!["repository engineering".to_owned()],
            risks: Vec::new(),
        }),
        Value::Object(fields) if fields.len() == 3 => {
            let workflow = fields.get("workflow")?.as_str()?.to_owned();
            let surfaces = strings(fields.get("surfaces")?)?;
            let risks = strings(fields.get("risks")?)?;
            if surfaces.iter().any(|item| item.is_empty())
                || risks.iter().any(|item| item.is_empty())
                || duplicate(&surfaces)
                || duplicate(&risks)
                || (!contract.routing.task_classes.contains(&workflow)
                    && !contract.routing.fail_closed_classes.contains(&workflow))
            {
                return None;
            }
            Some(Classification {
                workflow,
                surfaces,
                risks,
            })
        }
        _ => None,
    }
}

pub(super) fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(Value::as_str)
        .map(|item| item.map(ToOwned::to_owned))
        .collect()
}

pub(super) fn slot_is_valid(contract: &Contract, name: &str, slot: &Slot) -> bool {
    if let Slot::Omitted(omission) = slot {
        return contract.omission_reasons.contains(&omission.omitted.code)
            && !omission.omitted.reason.trim().is_empty();
    }
    let Slot::Present(present) = slot else {
        return false;
    };
    let value = &present.value;
    match name {
        "issue_pr_identity" => object(value, &["issue", "pr"], |item| {
            item.is_null() || item.is_u64()
        }),
        "owner_worktree" => object(value, &["owner", "worktree"], token),
        "base_head_sha" => object(value, &["base", "head"], token),
        "dirty_index_state" => object(value, &["dirty", "index"], Value::is_boolean),
        "unresolved_review_threads"
        | "verification"
        | "selected_references"
        | "authoritative_refresh_handles" => strings(value)
            .is_some_and(|items| items.iter().all(|item| token(&Value::String(item.clone())))),
        "checks" => {
            token(value)
                || value
                    .as_array()
                    .is_some_and(|items| items.iter().all(token))
        }
        "task_classification" => classification(slot, contract).is_some(),
        _ => token(value),
    }
}

fn object(value: &Value, keys: &[&str], valid: fn(&Value) -> bool) -> bool {
    value.as_object().is_some_and(|item| {
        item.len() == keys.len() && keys.iter().all(|key| item.get(*key).is_some_and(valid))
    })
}

fn token(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        !text.is_empty() && text.len() <= 256 && !text.chars().any(char::is_whitespace)
    })
}

fn duplicate(values: &[String]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    values.iter().any(|value| !seen.insert(value))
}

pub(super) fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    let value = super::super::routing_json::parse(text).map_err(anyhow::Error::msg)?;
    serde_json::from_value(value).map_err(anyhow::Error::from)
}

pub(super) fn digest(prefix: &str, bytes: &[u8]) -> String {
    format!("{prefix}:{:x}", Sha256::digest(bytes))
}
