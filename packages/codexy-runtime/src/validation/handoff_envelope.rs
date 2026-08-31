mod classification;
mod legacy;
mod replay;
mod schema;
use anyhow::{Result, bail, ensure};
pub use classification::{StableClassification, StructuredClassification};
pub use legacy::LegacyContext;
pub use schema::{
    BaseHeadSha, DirtyIndexState, HandoffEnvelope, HandoffEvent, HandoffVolatile, IssuePrIdentity,
    OwnerWorktree, ReviewThread, StableHandoff,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};
const MAX_ENVELOPE_BYTES: usize = 64 * 1024;
const POLICY: &str =
    include_str!("../../../../plugins/codexy/skills/orchestration/references/context-tiers.md");
const STABLE_PREFIX: &str = "codexy.handoff.stable.v1";
const VOLATILE_PREFIX: &str = "codexy.handoff.volatile.v1";
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OmissionReason {
    NotApplicable,
    NotCreated,
    ExternalSurfaceAbsent,
}
#[derive(Clone, Debug)]
pub struct HandoffAuthority {
    pub current_head: String,
    pub owner: String,
    pub worktree: String,
    stable: Option<StableHandoff>,
    lane: (IssuePrIdentity, String, String),
    seen_event_ids: Rc<RefCell<BTreeSet<String>>>,
}
impl HandoffAuthority {
    pub fn new(
        current_head: impl Into<String>,
        owner: impl Into<String>,
        worktree: impl Into<String>,
        identity: IssuePrIdentity,
        branch: impl Into<String>,
        base: impl Into<String>,
    ) -> Self {
        Self {
            current_head: current_head.into(),
            owner: owner.into(),
            worktree: worktree.into(),
            stable: None,
            lane: (identity, branch.into(), base.into()),
            seen_event_ids: Rc::new(RefCell::new(BTreeSet::new())),
        }
    }
    pub fn with_stable(mut self, mut stable: StableHandoff) -> Self {
        stable.policy_digest = stable_policy_digest();
        self.stable = Some(stable);
        self
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompactEnvelope {
    schema: String,
    stable_identity: String,
    volatile: HandoffVolatile,
    volatile_identity: String,
}
impl HandoffEnvelope {
    pub fn new(mut stable: StableHandoff, volatile: HandoffVolatile) -> Self {
        stable.policy_digest = stable_policy_digest();
        let stable_identity = schema::digest_value(&stable, STABLE_PREFIX);
        let volatile_identity = schema::digest_value(&volatile, VOLATILE_PREFIX);
        Self {
            schema: "codexy.handoff-envelope.v1".to_owned(),
            stable,
            volatile,
            stable_identity,
            volatile_identity,
        }
    }
    pub fn canonical_json(&self) -> Result<String> {
        serde_json::to_string(&CompactEnvelope {
            schema: self.schema.clone(),
            stable_identity: self.stable_identity.clone(),
            volatile: self.volatile.clone(),
            volatile_identity: self.volatile_identity.clone(),
        })
        .map_err(anyhow::Error::from)
    }
}
pub fn stable_policy_digest() -> String {
    format!("sha256:{:x}", Sha256::digest(POLICY.as_bytes()))
}
pub fn canonicalize_handoff(text: &str) -> Result<String> {
    ensure!(
        !text.is_empty() && text.len() <= MAX_ENVELOPE_BYTES,
        "bounded handoff"
    );
    let value = super::routing_json::parse(text).map_err(anyhow::Error::msg)?;
    let compact: CompactEnvelope = serde_json::from_value(value).map_err(anyhow::Error::from)?;
    ensure!(
        compact.schema == "codexy.handoff-envelope.v1"
            && compact.volatile_identity
                == schema::digest_value(&compact.volatile, VOLATILE_PREFIX),
        "compact envelope"
    );
    serde_json::to_string(&compact).map_err(anyhow::Error::from)
}
pub fn validate_handoff(text: &str, authority: &HandoffAuthority) -> Result<HandoffEnvelope> {
    replay::validate_single(text, authority)
}
pub fn validate_handoff_batch(
    texts: &[&str],
    authority: &HandoffAuthority,
) -> Result<Vec<HandoffEnvelope>> {
    replay::validate_batch(texts, authority)
}
pub fn migrate_legacy_handoff(text: &str, context: &LegacyContext) -> Result<String> {
    let envelope = legacy::migrate(text, context)?;
    validate_intrinsic(&envelope)?;
    envelope.canonical_json()
}
fn parse_envelope(text: &str, cached_stable: Option<&StableHandoff>) -> Result<HandoffEnvelope> {
    ensure!(
        !text.is_empty() && text.len() <= MAX_ENVELOPE_BYTES,
        "handoff envelope exceeds the bounded input size"
    );
    let value = super::routing_json::parse(text).map_err(anyhow::Error::msg)?;
    let compact: CompactEnvelope = serde_json::from_value(value).map_err(anyhow::Error::from)?;
    let Some(cached_stable) = cached_stable else {
        bail!("compact handoff requires the referenced stable body");
    };
    let stable = StableHandoff {
        policy_digest: stable_policy_digest(),
        ..cached_stable.clone()
    };
    ensure!(
        compact.stable_identity == schema::digest_value(&stable, STABLE_PREFIX),
        "handoff stable identity conflicts with the referenced stable body"
    );
    Ok(HandoffEnvelope {
        schema: compact.schema,
        stable,
        volatile: compact.volatile,
        stable_identity: compact.stable_identity,
        volatile_identity: compact.volatile_identity,
    })
}
fn validate_intrinsic(envelope: &HandoffEnvelope) -> Result<()> {
    ensure!(envelope.schema == "codexy.handoff-envelope.v1", "schema");
    ensure!(
        envelope.stable.policy_digest == stable_policy_digest(),
        "policy digest"
    );
    validate_stable(&envelope.stable)?;
    schema::validate_volatile(&envelope.volatile)?;
    ensure!(
        envelope.stable_identity == schema::digest_value(&envelope.stable, STABLE_PREFIX)
            && envelope.volatile_identity
                == schema::digest_value(&envelope.volatile, VOLATILE_PREFIX),
        "envelope identity"
    );
    Ok(())
}
fn validate_stable(stable: &StableHandoff) -> Result<()> {
    ensure!(
        matches!(
            stable.workflow_profile.as_str(),
            "light" | "standard" | "strict"
        ),
        "handoff has an unknown workflow profile"
    );
    let route = classification::route(&stable.task_classification)?;
    schema::validate_unique_strings(&stable.selected_references, "selected_references")?;
    stable
        .selected_references
        .iter()
        .try_for_each(|item| schema::token(item, "selected reference"))?;
    ensure!(
        stable.selected_references == route.references,
        "handoff selected references disagree with the canonical route"
    );
    if route.fail_closed {
        ensure!(
            stable.workflow_profile == "strict",
            "fail-closed handoff must use the strict profile"
        );
    }
    schema::token(&stable.workflow_profile, "workflow profile")?;
    Ok(())
}
