//! Match the immediately preceding policy without relaxing live lane authority.

use anyhow::{Result, bail, ensure};

use super::{
    HandoffEnvelope, STABLE_PREFIX, StableHandoff, classification, schema, stable_policy_digest,
};

// This is the exact policy distributed with the preceding 2.2.1 capsule bridge.
const PREVIOUS_POLICY: &str =
    "sha256:54c9f40fdc617c4e6dc3082cf5f19ef7e7c6babcf76e7551b56069a2c0bd8605";

fn references(stable: &StableHandoff) -> Result<(Vec<String>, Vec<String>)> {
    let current = classification::route(&stable.task_classification)?.references;
    let previous = current
        .iter()
        .map(|item| {
            if item == "self-improvement" {
                "dreaming".to_owned()
            } else {
                item.clone()
            }
        })
        .collect();
    Ok((current, previous))
}

pub(super) fn resolve_stable(cached: &StableHandoff, identity: &str) -> Result<StableHandoff> {
    let (current, previous) = references(cached)?;
    // The separately trusted authority must itself carry one complete canonical
    // route; an arbitrary or weakened selection cannot be normalized into one.
    ensure!(
        cached.selected_references == current || cached.selected_references == previous,
        "trusted handoff references disagree with either supported route"
    );
    for (digest, selected) in [
        (stable_policy_digest(), current),
        (PREVIOUS_POLICY.to_owned(), previous),
    ] {
        let stable = StableHandoff {
            policy_digest: digest,
            selected_references: selected,
            ..cached.clone()
        };
        if schema::digest_value(&stable, STABLE_PREFIX) == identity {
            return Ok(stable);
        }
    }
    bail!("handoff stable identity conflicts with the referenced stable body")
}

pub(super) fn validate_policy(stable: &StableHandoff) -> Result<()> {
    let (current, previous) = references(stable)?;
    ensure!(
        (stable.policy_digest == stable_policy_digest() && stable.selected_references == current)
            || (stable.policy_digest == PREVIOUS_POLICY && stable.selected_references == previous),
        "handoff policy digest or selected references disagree with the supported route"
    );
    Ok(())
}

pub(super) fn project_previous(mut envelope: HandoffEnvelope) -> Result<HandoffEnvelope> {
    let (_, previous) = references(&envelope.stable)?;
    envelope.stable.policy_digest = PREVIOUS_POLICY.into();
    envelope.stable.selected_references = previous;
    envelope.stable_identity = schema::digest_value(&envelope.stable, STABLE_PREFIX);
    validate_policy(&envelope.stable)?;
    Ok(envelope)
}

pub(super) fn requires_previous_route(stable: &StableHandoff) -> Result<()> {
    let (_, previous) = references(stable)?;
    ensure!(
        stable.selected_references == previous,
        "rollback authority must carry the previous canonical route"
    );
    Ok(())
}
