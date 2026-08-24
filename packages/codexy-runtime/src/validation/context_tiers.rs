mod contract_validation;
#[path = "context_tiers/schema.rs"]
mod schema;
mod surface_routing;

use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

use self::schema::{Contract, CurrentState, Envelope, digest, parse};

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
                .is_none_or(|slot| !schema::slot_is_valid(&contract, &field.name, slot))
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
    Ok(surface_routing::validate_envelope(
        &contract,
        &envelope,
        &current,
        &expected_identities,
    ))
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
    contract_validation::validate(&contract, plugin_root)?;
    Ok((contract, text))
}
