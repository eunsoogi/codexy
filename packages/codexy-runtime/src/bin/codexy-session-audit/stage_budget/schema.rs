use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

pub(crate) type Stage = String;
pub(crate) type OwnerKind = String;
pub(crate) type BudgetUnit = String;
pub(crate) type ReviewerState = String;
pub(crate) type ExternalGate = String;
pub(crate) type MeasureAvailability = String;
pub(crate) type OversizedKind = String;
pub(crate) type OversizedState = String;
pub(crate) type Decision = String;
pub(crate) type NextAction = String;
macro_rules! closed_struct {
    ($name:ident; $($field:ident: $ty:ty),+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub(crate) struct $name { $(pub(crate) $field: $ty),+ }
    };
}
closed_struct!(StageBudgetReceipt;
    schema: String, metadata_only: bool, stage: Stage, stage_sequence: u64,
    previous_receipt_identity: Option<String>, receipt_identity: String, continuity: Continuity,
    owner: Owner, identity: Identity, units: Units, safety: Safety, proof: ProofState,
    limits: Limits, usage: Usage, events: Events, measures: Measures,
    oversized_result: Option<OversizedResult>, decision: Decision, next_action: NextAction
);
closed_struct!(Owner; kind: OwnerKind, id: String);
closed_struct!(Identity; stable: String, volatile: String);
closed_struct!(Continuity; previous: Option<PreviousReceipt>, cumulative_replay_events: u64);
closed_struct!(PreviousReceipt;
    stage: Stage, stage_sequence: u64, previous_receipt_identity: Option<String>,
    receipt_identity: String, owner: Owner, identity: Identity, safety: Safety,
    proof: ProofState, limits: Limits, usage: Usage, events: Events,
    oversized_result: Option<OversizedResult>,
    cumulative_replay_events: u64
);
closed_struct!(Units; context: BudgetUnit, tool_output: BudgetUnit);
closed_struct!(Safety;
    issue_pr_identity: IssuePrIdentity, owner_worktree: OwnerWorktree, base_head_sha: BaseHeadSha,
    dirty_index_state: DirtyIndexState, checks: Vec<Check>, unresolved_review_threads: Vec<String>,
    selected_reviewer_state: ReviewerState, verification: Vec<String>, external_gate: ExternalGate
);
closed_struct!(IssuePrIdentity; issue: String, pr: Option<String>);
closed_struct!(OwnerWorktree; owner_thread_id: String, branch: String, worktree: String);
closed_struct!(BaseHeadSha; base: String, head: String);
closed_struct!(DirtyIndexState; dirty: bool, index: bool);
closed_struct!(Check; name: String, state: String);
closed_struct!(ProofState; goal: String, plan: String, verification: String);
closed_struct!(Limits; context_bytes: u64, tool_output_bytes: u64, replay_events: u64, turns: u64, tool_calls: u64);
closed_struct!(Usage; context_bytes: u64, tool_output_bytes: u64, turns: u64, tool_calls: u64);
closed_struct!(Events; identities: Vec<String>, unchanged_waits: u64, full_state_replays: u64, oversized_preview_reads: u64);
closed_struct!(Measures;
    input_tokens: Measure<u64>, wall_time_ms: Measure<u64>, observed_cost_usd: Measure<f64>,
    tool_input_bytes: Measure<u64>, tool_output_bytes: Measure<u64>, cache_input_tokens: Measure<u64>
);
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Measure<T> {
    pub(crate) availability: MeasureAvailability,
    pub(crate) value: Option<T>,
    pub(crate) reason: Option<String>,
}
closed_struct!(OversizedResult; kind: OversizedKind, identity: String, bytes: u64, state: OversizedState, body_replayed: bool);

pub(crate) fn validate_continuity(r: &StageBudgetReceipt, replay: u64) -> Result<()> {
    if r.continuity.cumulative_replay_events != replay {
        bail!("cumulative replay accounting does not match this receipt");
    }
    if r.receipt_identity
        != anchor_identity(&AnchorInput {
            stage: r.stage.as_str(),
            sequence: r.stage_sequence,
            previous: r.previous_receipt_identity.as_deref(),
            owner: &r.owner,
            identity: &r.identity,
            safety: &r.safety,
            proof: &r.proof,
            limits: &r.limits,
            usage: &r.usage,
            events: &r.events,
            oversized_result: r.oversized_result.as_ref(),
            replay,
        })?
    {
        bail!("receipt identity does not authenticate cumulative state");
    }
    if r.stage_sequence == 1 {
        if r.continuity.previous.is_some() {
            bail!("the first stage receipt cannot have prior cumulative state");
        }
        return Ok(());
    }
    let p = r
        .continuity
        .previous
        .as_ref()
        .context("continued stages require prior cumulative state")?;
    if p.stage_sequence.checked_add(1) != Some(r.stage_sequence)
        || r.previous_receipt_identity.as_deref() != Some(p.receipt_identity.as_str())
    {
        bail!("continued stage identity does not match authenticated prior state");
    }
    validate_previous(p)?;
    if [
        r.usage.context_bytes < p.usage.context_bytes,
        r.usage.tool_output_bytes < p.usage.tool_output_bytes,
        r.usage.turns < p.usage.turns,
        r.usage.tool_calls < p.usage.tool_calls,
        replay < p.cumulative_replay_events,
    ]
    .into_iter()
    .any(|v| v)
    {
        bail!("continued receipt usage cannot decrease or renew prior cumulative state");
    }
    if [
        r.limits.context_bytes != p.limits.context_bytes,
        r.limits.tool_output_bytes != p.limits.tool_output_bytes,
        r.limits.replay_events != p.limits.replay_events,
        r.limits.turns != p.limits.turns,
        r.limits.tool_calls != p.limits.tool_calls,
    ]
    .into_iter()
    .any(|v| v)
    {
        bail!("continued receipts cannot renew finite stage limits");
    }
    if ((p.stage == "selected-review" && r.stage == "wait")
        || (p.stage == "wait" && r.stage == "wait" && p.owner.kind == "selected-reviewer"))
        && r.owner.id != p.owner.id
    {
        bail!("reviewer wait must preserve the same reviewer owner");
    }
    if r.identity.stable != p.identity.stable
        || r.owner != p.owner
        || r.safety.issue_pr_identity != p.safety.issue_pr_identity
        || r.safety.owner_worktree != p.safety.owner_worktree
        || r.safety.base_head_sha != p.safety.base_head_sha
        || r.proof.goal != p.proof.goal
        || r.proof.plan != p.proof.plan
        || r.identity.volatile == p.identity.volatile
        || !r.events.identities.starts_with(&p.events.identities)
        || r.events.identities[p.events.identities.len()..]
            .iter()
            .any(|id| {
                id == &p.identity.volatile
                    || p.events.identities.iter().any(|prior_id| prior_id == id)
            })
    {
        bail!(
            "continued receipt changed stable ownership or proof identity, or reused event history"
        );
    }
    Ok(())
}
struct AnchorInput<'a> {
    stage: &'a str,
    sequence: u64,
    previous: Option<&'a str>,
    owner: &'a Owner,
    identity: &'a Identity,
    safety: &'a Safety,
    proof: &'a ProofState,
    limits: &'a Limits,
    usage: &'a Usage,
    events: &'a Events,
    oversized_result: Option<&'a OversizedResult>,
    replay: u64,
}
fn anchor_identity(input: &AnchorInput<'_>) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            input.stage,
            input.sequence,
            input.previous,
            input.owner,
            input.identity,
            input.safety,
            input.proof,
            input.limits,
            input.usage,
            input.events,
            input.oversized_result,
            input.replay
        ))?)
    ))
}
fn validate_previous(p: &PreviousReceipt) -> Result<()> {
    super::validation::validate_owner(&p.owner)?;
    super::validation::validate_identity(&p.identity)?;
    super::validation::validate_safety(&p.safety)?;
    super::validation::validate_proof(&p.proof)?;
    super::validation::validate_limits(&p.limits)?;
    super::validation::validate_events(&p.events)?;
    super::validation::validate_oversized_result(
        p.oversized_result.as_ref(),
        &p.events,
        &p.limits,
        &p.usage,
    )?;
    if p.stage_sequence == 0
        || !["root", "child", "selected-reviewer"].contains(&p.owner.kind.as_str())
        || !super::validation::stage_owner_valid(&p.stage, &p.owner)
    {
        bail!("prior stage owner or sequence is invalid");
    }
    super::validation::validate_digest(&p.receipt_identity)?;
    if p.stage_sequence > 1 {
        super::validation::validate_digest(
            p.previous_receipt_identity
                .as_deref()
                .context("continued prior receipts require a previous receipt identity")?,
        )?;
    } else if p.previous_receipt_identity.is_some() {
        bail!("the first prior receipt cannot have a previous identity");
    }
    if p.receipt_identity
        != anchor_identity(&AnchorInput {
            stage: &p.stage,
            sequence: p.stage_sequence,
            previous: p.previous_receipt_identity.as_deref(),
            owner: &p.owner,
            identity: &p.identity,
            safety: &p.safety,
            proof: &p.proof,
            limits: &p.limits,
            usage: &p.usage,
            events: &p.events,
            oversized_result: p.oversized_result.as_ref(),
            replay: p.cumulative_replay_events,
        })?
    {
        bail!("prior receipt identity does not authenticate cumulative state");
    }
    Ok(())
}
