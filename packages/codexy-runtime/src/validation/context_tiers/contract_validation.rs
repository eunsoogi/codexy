use std::{collections::BTreeSet, path::Path};

use anyhow::{Result, bail};

use super::schema::Contract;
use crate::validation::FORWARDED_CONTEXT_TYPES;

const SAFETY_FIELDS: [&str; 10] = [
    "issue_pr_identity",
    "owner_worktree",
    "base_head_sha",
    "dirty_index_state",
    "checks",
    "unresolved_review_threads",
    "selected_reviewer_state",
    "verification",
    "external_gate",
    "next_action",
];
const TASK_CLASSES: [&str; 10] = [
    "orchestration/lane setup",
    "implementation",
    "review response",
    "GitHub/merge",
    "validation/QA",
    "documentation/skill authoring",
    "plugin/release",
    "investigation/debugging",
    "issue/intake only",
    "other",
];
const SURFACES: [&str; 7] = [
    "repository engineering",
    "GitHub",
    "browser/desktop",
    "documents/artifacts",
    "spreadsheets/data",
    "research/wiki",
    "read-only/local",
];
const RISKS: [&str; 5] = [
    "mixed",
    "security",
    "permission",
    "destructive",
    "external_mutation",
];

pub(super) fn validate(contract: &Contract, plugin_root: &Path) -> Result<()> {
    if contract.schema != "codexy.context-tiers.v1"
        || contract.tier_order != ["always_on", "task_selected", "event_delta", "refresh_only"]
        || contract.identity_order != ["stable", "volatile"]
    {
        bail!("context contract has unknown tiers or identity order");
    }
    let authority_ids = authority_ids(contract, plugin_root)?;
    validate_fields(contract, &authority_ids)?;
    validate_profiles(contract)?;
    validate_routes(contract, &authority_ids)?;
    validate_semantics(contract)?;
    Ok(())
}

fn authority_ids(contract: &Contract, plugin_root: &Path) -> Result<BTreeSet<String>> {
    let ids = contract
        .authorities
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let paths = contract
        .authorities
        .iter()
        .map(|item| item.path.clone())
        .collect::<BTreeSet<_>>();
    if ids.len() != contract.authorities.len()
        || paths.len() != contract.authorities.len()
        || contract
            .authorities
            .iter()
            .any(|item| !plugin_root.join(&item.path).is_file())
    {
        bail!("context authorities must be unique and resolve inside the plugin");
    }
    Ok(ids)
}

fn validate_fields(contract: &Contract, authority_ids: &BTreeSet<String>) -> Result<()> {
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
                || !authority_ids.contains(&field.authority)
        })
        || SAFETY_FIELDS.iter().any(|required| {
            !contract.retained_fields.iter().any(|field| {
                field.name == *required
                    && field.tier == "always_on"
                    && field.identity == "volatile"
                    && field.safety_invariant
                    && field.budget_exempt
            })
        })
    {
        bail!("retained fields must have one closed tier, identity, and authority");
    }
    Ok(())
}

fn validate_profiles(contract: &Contract) -> Result<()> {
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
    Ok(())
}

fn validate_routes(contract: &Contract, authority_ids: &BTreeSet<String>) -> Result<()> {
    let routing = &contract.routing;
    if routing.task_classes != TASK_CLASSES
        || routing.task_reference_routes.len() != TASK_CLASSES.len()
        || routing.surface_names != SURFACES
        || routing.surface_reference_routes.len() != SURFACES.len()
        || routing.surface_non_applicable_fields.len() != SURFACES.len()
        || routing.risk_names != RISKS
        || routing.risk_reference_routes.len() != RISKS.len()
        || routing.fallback_reference_route
            != ["workflow_profiles", "task_classification", "child_routing"]
        || routing.fallback_authority != "child_routing"
        || routing.fail_closed_classes
            != [
                "unknown",
                "ambiguous",
                "high_risk",
                "security",
                "permission",
                "release",
            ]
        || !routing.fail_closed_preserves_all_safety_fields
    {
        bail!("task, surface, and risk routes must remain closed");
    }
    for route in routing
        .task_reference_routes
        .values()
        .chain(routing.surface_reference_routes.values())
        .chain(routing.risk_reference_routes.values())
        .chain(std::iter::once(&routing.fallback_reference_route))
    {
        if !route_values_are_authority_backed(route, authority_ids) {
            bail!("task, surface, and risk routes must be authority-backed and ordered");
        }
    }
    let field_names = contract
        .retained_fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();
    if routing
        .surface_non_applicable_fields
        .values()
        .flatten()
        .any(|field| {
            !field_names.contains(field.as_str()) || !SAFETY_FIELDS.contains(&field.as_str())
        })
    {
        bail!("surface applicability must use known safety fields");
    }
    Ok(())
}

fn route_values_are_authority_backed(route: &[String], authority_ids: &BTreeSet<String>) -> bool {
    let unique = route.iter().collect::<BTreeSet<_>>();
    unique.len() == route.len() && route.iter().all(|item| authority_ids.contains(item))
}

fn validate_semantics(contract: &Contract) -> Result<()> {
    let fields = contract
        .retained_fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<BTreeSet<_>>();
    let ordered = contract
        .ordering
        .stable_fields
        .iter()
        .chain(&contract.ordering.volatile_fields)
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let budget = &contract.budget_semantics;
    if ordered != fields
        || contract.ordering.stable_prefix == contract.ordering.volatile_prefix
        || contract.ordering.stable_identity
            != "sha256_exact_contract_bytes_and_typed_stable_fields"
        || contract.ordering.volatile_identity != "sha256_typed_envelope_in_field_order"
        || budget.context_unit != "utf8_bytes_after_serialization"
        || budget.output_unit != "utf8_bytes_emitted"
        || budget.limit_source != "positive_consumer_supplied_per_stage"
        || budget.context_floor != ["always_on", "applicable_task_selected"]
        || budget.cache_metadata != "unavailable_not_zero"
        || budget.cache_savings_claims != "prohibited_without_runtime_evidence"
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
