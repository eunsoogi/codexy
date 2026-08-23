use super::schema;
use anyhow::{Context as _, Result, bail};

#[path = "validation/metadata.rs"]
mod metadata;

pub(crate) fn validate_receipt(r: &schema::StageBudgetReceipt) -> Result<()> {
    if r.schema != "codexy.stage-budget.v1" || !r.metadata_only {
        bail!("stage budget receipt must be closed metadataOnly=true");
    }
    if r.stage_sequence == 0 || (r.stage_sequence == 1 && r.previous_receipt_identity.is_some()) {
        bail!("stage sequence and previous identity are invalid");
    }
    if r.stage_sequence > 1 {
        metadata::validate_digest(
            r.previous_receipt_identity
                .as_deref()
                .context("continued stages require a previous receipt identity")?,
        )?;
    }
    validate_owner(&r.owner)?;
    if !stage_owner_valid(&r.stage, &r.owner) {
        bail!("stage owner does not match the closed stage contract");
    }
    for (value, allowed, label) in [
        (
            &r.units.context,
            &["utf8_bytes_after_serialization"][..],
            "context unit",
        ),
        (
            &r.units.tool_output,
            &["utf8_bytes_emitted"][..],
            "tool output unit",
        ),
        (
            &r.safety.selected_reviewer_state,
            &[
                "pending",
                "running",
                "pass",
                "block",
                "unobservable",
                "not-applicable",
            ][..],
            "reviewer state",
        ),
        (
            &r.safety.external_gate,
            &["none", "pending", "pass", "blocked", "not-applicable"][..],
            "external gate",
        ),
        (
            &r.proof.goal,
            &["active", "complete", "idle"][..],
            "goal state",
        ),
        (
            &r.proof.plan,
            &["active", "complete", "idle"][..],
            "plan state",
        ),
        (
            &r.proof.verification,
            &["pending", "pass", "fail", "unavailable"][..],
            "verification state",
        ),
        (
            &r.decision,
            &["continue", "compact", "stop_and_handoff"][..],
            "decision",
        ),
        (
            &r.next_action,
            &[
                "continue-stage",
                "compact-context",
                "wait-for-event",
                "handoff-parent",
            ][..],
            "next action",
        ),
    ] {
        metadata::closed(value, allowed, label)?;
    }
    if r.stage == "selected-review"
        && (r.safety.selected_reviewer_state == "not-applicable"
            || r.safety.external_gate == "none"
            || r.next_action == "continue-stage")
    {
        bail!("selected-review requires a reviewer gate and one bounded action");
    }
    if r.safety.selected_reviewer_state == "running"
        && (r.stage != "selected-review" && r.stage != "wait"
            || r.owner.kind != "selected-reviewer")
    {
        bail!("a running selected reviewer must own selected-review or wait");
    }
    if r.safety.selected_reviewer_state == "running" && r.safety.external_gate != "pending" {
        bail!("a running selected review requires a pending external gate");
    }
    validate_identity(&r.identity)?;
    validate_safety(&r.safety)?;
    validate_proof(&r.proof)?;
    validate_limits(&r.limits)?;
    metadata::validate_events(&r.events)?;
    metadata::validate_measures(
        &r.measures,
        r.usage.tool_output_bytes,
        r.oversized_result.as_ref(),
    )?;
    metadata::validate_oversized_result(r.oversized_result.as_ref(), &r.events, &r.limits, &r.usage)
}

pub(super) fn validate_events(events: &schema::Events) -> Result<()> {
    metadata::validate_events(events)
}

pub(super) fn stage_owner_valid(stage: &str, owner: &schema::Owner) -> bool {
    match stage {
        "root-planning" | "parent-integration" => owner.kind == "root",
        "child-implementation" | "repair" => owner.kind == "child",
        "selected-review" => owner.kind == "selected-reviewer",
        "wait" => true,
        _ => false,
    }
}

pub(super) fn validate_oversized_result(
    result: Option<&schema::OversizedResult>,
    events: &schema::Events,
    limits: &schema::Limits,
    usage: &schema::Usage,
) -> Result<()> {
    metadata::validate_oversized_result(result, events, limits, usage)
}

pub(super) fn validate_digest(value: &str) -> Result<()> {
    metadata::validate_digest(value)
}

pub(super) fn validate_owner(owner: &schema::Owner) -> Result<()> {
    metadata::closed(
        &owner.kind,
        &["root", "child", "selected-reviewer"],
        "owner kind",
    )?;
    metadata::validate_token(&owner.id, "owner id")
}

pub(super) fn validate_identity(identity: &schema::Identity) -> Result<()> {
    metadata::validate_token(&identity.stable, "stable identity")?;
    metadata::validate_token(&identity.volatile, "volatile identity")
}

pub(super) fn validate_proof(proof: &schema::ProofState) -> Result<()> {
    metadata::closed(&proof.goal, &["active", "complete", "idle"], "goal state")?;
    metadata::closed(&proof.plan, &["active", "complete", "idle"], "plan state")?;
    metadata::closed(
        &proof.verification,
        &["pending", "pass", "fail", "unavailable"],
        "verification state",
    )
}

pub(super) fn validate_limits(limits: &schema::Limits) -> Result<()> {
    if [
        limits.context_bytes,
        limits.tool_output_bytes,
        limits.replay_events,
        limits.turns,
        limits.tool_calls,
    ]
    .contains(&0)
    {
        bail!("stage budget limits must be positive and finite");
    }
    Ok(())
}

pub(super) fn validate_safety(safety: &schema::Safety) -> Result<()> {
    for (value, label) in [
        (&safety.issue_pr_identity.issue, "issue identity"),
        (
            &safety.owner_worktree.owner_thread_id,
            "owner thread identity",
        ),
        (&safety.owner_worktree.branch, "branch identity"),
        (&safety.owner_worktree.worktree, "worktree identity"),
    ] {
        metadata::validate_token(value, label)?;
    }
    if let Some(pr) = &safety.issue_pr_identity.pr {
        metadata::validate_token(pr, "pull request identity")?;
    }
    metadata::validate_sha(&safety.base_head_sha.base, "base sha")?;
    metadata::validate_sha(&safety.base_head_sha.head, "head sha")?;
    if [
        safety.checks.len(),
        safety.unresolved_review_threads.len(),
        safety.verification.len(),
    ]
    .into_iter()
    .any(|length| length > 64)
    {
        bail!("safety metadata lists must remain bounded");
    }
    for check in &safety.checks {
        metadata::validate_token(&check.name, "check name")?;
        metadata::closed(
            &check.state,
            &["pending", "passing", "failing", "unavailable"],
            "check state",
        )?;
    }
    for identity in safety
        .unresolved_review_threads
        .iter()
        .chain(&safety.verification)
    {
        metadata::validate_token(identity, "safety metadata identity")?;
    }
    metadata::closed(
        &safety.selected_reviewer_state,
        &[
            "pending",
            "running",
            "pass",
            "block",
            "unobservable",
            "not-applicable",
        ],
        "reviewer state",
    )?;
    metadata::closed(
        &safety.external_gate,
        &["none", "pending", "pass", "blocked", "not-applicable"],
        "external gate",
    )
}
