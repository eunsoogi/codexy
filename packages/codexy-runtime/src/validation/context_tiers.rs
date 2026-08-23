#[path = "context_tiers/schema.rs"]
mod schema;

use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

use self::schema::{Contract, CurrentState, Envelope, Slot, digest, parse, slot_string};

const CONTRACT_PATH: &str = "skills/orchestration/references/context-tiers.json";
const CANONICAL_CONTRACT: &str =
    include_str!("../../../../plugins/codexy/skills/orchestration/references/context-tiers.json");
pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    load(plugin_root)
        .err()
        .map_or_else(Vec::new, |error| vec![error.to_string()])
}

pub(crate) fn identities(plugin_root: &Path, current_text: &str) -> Result<[String; 2]> {
    let (contract, contract_text) = load(plugin_root)?;
    let current: CurrentState = parse(current_text)?;
    if current.schema != "codexy.context-current-state.v1"
        || current.slots.len() != contract.retained_fields.len()
        || contract.retained_fields.iter().any(|field| {
            current
                .slots
                .get(&field.name)
                .is_none_or(|slot| !valid_slot(&contract, &field.name, slot))
        })
    {
        bail!("current context state is incomplete or has an invalid value shape");
    }
    let ordered = |names: &[String]| -> Result<Vec<_>> {
        names
            .iter()
            .map(|name| {
                current
                    .slots
                    .get(name)
                    .map(|slot| (name.clone(), slot.clone()))
                    .ok_or_else(|| anyhow::anyhow!("missing ordered context field {name}"))
            })
            .collect()
    };
    let stable = serde_json::to_vec(&(
        contract_text.as_bytes(),
        ordered(&contract.ordering.stable_fields)?,
    ))?;
    let volatile = serde_json::to_vec(&ordered(&contract.ordering.volatile_fields)?)?;
    Ok([
        digest(&contract.ordering.stable_prefix, &stable),
        digest(&contract.ordering.volatile_prefix, &volatile),
    ])
}

pub(crate) fn validate_envelope(
    plugin_root: &Path,
    envelope_text: &str,
    current_text: &str,
) -> Result<Vec<String>> {
    let (contract, _) = load(plugin_root)?;
    let envelope: Envelope = parse(envelope_text)?;
    let current: CurrentState = parse(current_text)?;
    let expected_identities = identities(plugin_root, current_text)?;
    let mut errors = Vec::new();
    if envelope.schema != "codexy.context-envelope.v1" {
        errors.push("context envelope or current state has an unsupported schema".to_owned());
    }
    let routing = &contract.routing;
    let ordinary = routing.task_classes.contains(&envelope.task_class);
    let fail_closed = routing.fail_closed_classes.contains(&envelope.task_class);
    if !ordinary && !fail_closed {
        errors.push("context envelope has an unknown task or risk class".to_owned());
    }
    if fail_closed
        && (envelope.profile != "strict"
            || envelope.route_authority.as_deref() != Some(routing.fallback_authority.as_str()))
    {
        errors.push("risk context must fail closed through the routing authority".to_owned());
    }
    if ordinary && envelope.route_authority.is_some() {
        errors.push("ordinary context must not invent a risk-route authority".to_owned());
    }
    if ordinary {
        let expected = &routing.task_reference_routes[&envelope.task_class];
        if !selected_refs(&envelope.slots, expected) {
            errors.push("ordinary context selected references disagree with its route".to_owned());
        }
    }
    if !contract.profile_matrix.contains_key(&envelope.profile) {
        errors.push("context envelope has an unknown workflow profile".to_owned());
    }
    if slot_string(&envelope.slots, "workflow_profile") != Some(envelope.profile.as_str())
        || slot_string(&envelope.slots, "task_classification") != Some(envelope.task_class.as_str())
    {
        errors.push("context envelope classification slots disagree with its routing".to_owned());
    }
    if envelope.stable_identity != expected_identities[0]
        || envelope.volatile_identity != expected_identities[1]
    {
        errors.push("context envelope identities do not match deterministic state".to_owned());
    }
    for field in &contract.retained_fields {
        compare(
            &contract,
            &envelope,
            &current,
            field,
            fail_closed,
            &mut errors,
        );
    }
    let unknown = |name: &String| {
        !contract
            .retained_fields
            .iter()
            .any(|field| field.name == *name)
    };
    if envelope.slots.keys().any(unknown) {
        errors.push("context state contains an unknown retained field".to_owned());
    }
    Ok(errors)
}

fn compare(
    contract: &Contract,
    envelope: &Envelope,
    current: &CurrentState,
    field: &schema::RetainedField,
    fail_closed: bool,
    errors: &mut Vec<String>,
) {
    let retained = envelope.slots.get(&field.name);
    let authoritative = current.slots.get(&field.name);
    let Some((retained, authoritative)) = retained.zip(authoritative) else {
        errors.push(format!("missing retained field {}", field.name));
        return;
    };
    if retained != authoritative {
        errors.push(format!("stale retained field {}", field.name));
    }
    if !valid_slot(contract, &field.name, authoritative) {
        errors.push(format!("invalid value shape for {}", field.name));
    }
    let Slot::Omitted(omission) = retained else {
        return;
    };
    let omitted = &omission.omitted;
    let policy = contract
        .profile_matrix
        .get(&envelope.profile)
        .and_then(|tiers| tiers.get(&field.tier))
        .map_or("invalid", String::as_str);
    let typed =
        contract.omission_reasons.contains(&omitted.code) && !omitted.reason.trim().is_empty();
    let permitted = match policy {
        "when_applicable" | "typed_omission_when_not_applicable" => typed,
        "required_before_authoritative_action" => typed && !envelope.action_allowed,
        _ => false,
    };
    if !permitted || (fail_closed && field.safety_invariant) {
        errors.push(format!("unauthorized omission for {}", field.name));
    }
    if omitted.code == "external_surface_absent" && envelope.action_allowed {
        errors.push("unavailable external state must fail closed".to_owned());
    }
}

fn load(plugin_root: &Path) -> Result<(Contract, String)> {
    let path = plugin_root.join(CONTRACT_PATH);
    let text = std::fs::read_to_string(&path)?;
    let candidate: Value = parse(&text)?;
    let canonical: Value = parse(CANONICAL_CONTRACT)?;
    if candidate != canonical {
        bail!("context tier contract differs from the closed canonical schema");
    }
    let contract: Contract = serde_json::from_value(candidate)?;
    schema::validate(&contract, plugin_root)?;
    Ok((contract, text))
}

fn selected_refs(slots: &std::collections::BTreeMap<String, Slot>, expected: &[String]) -> bool {
    let Some(Slot::Present(present)) = slots.get("selected_references") else {
        return false;
    };
    present.value.as_array().is_some_and(|items| {
        items.len() == expected.len()
            && items
                .iter()
                .zip(expected)
                .all(|(item, name)| item.as_str() == Some(name))
    })
}

fn valid_slot(contract: &Contract, name: &str, slot: &Slot) -> bool {
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
        | "authoritative_refresh_handles" => strings(value),
        "checks" => token(value) || strings(value),
        "task_classification" => value.as_str().is_some_and(|task| {
            contract
                .routing
                .task_classes
                .iter()
                .chain(&contract.routing.fail_closed_classes)
                .any(|known| known == task)
        }),
        _ => token(value),
    }
}

fn object(value: &Value, keys: &[&str], valid: fn(&Value) -> bool) -> bool {
    value.as_object().is_some_and(|item| {
        item.len() == keys.len() && keys.iter().all(|key| item.get(*key).is_some_and(valid))
    })
}

fn strings(value: &Value) -> bool {
    matches!(value, Value::Array(items) if items.iter().all(token))
}

fn token(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        !text.is_empty() && text.len() <= 256 && !text.chars().any(char::is_whitespace)
    })
}
