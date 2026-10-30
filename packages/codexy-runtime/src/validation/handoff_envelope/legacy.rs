mod current;
mod delta;
mod fields;
use super::schema::{
    BaseHeadSha, DirtyIndexState, HandoffEnvelope, HandoffEvent, HandoffVolatile, IssuePrIdentity,
    OwnerWorktree, ReviewThread, StableHandoff,
};
use super::{OmissionReason, schema};
use anyhow::{Result, bail, ensure};
use fields::fields;
use std::collections::BTreeMap;
const MAX_LEGACY_BYTES: usize = 64 * 1024;
#[derive(Clone, Debug)]
pub struct LegacyContext {
    pub stable: StableHandoff,
    pub owner: String,
    pub branch: String,
    pub worktree: String,
    pub base: String,
    pub dirty_index_state: DirtyIndexState,
    pub checks: Vec<String>,
    pub unresolved_review_threads: Vec<ReviewThread>,
    pub selected_reviewer_state: String,
    pub verification: Vec<String>,
    pub active_obligation: String,
    pub external_gate: String,
    pub child_task: Option<String>,
    pub parent_task: Option<String>,
    pub preserved_artifacts: Option<String>,
    pub authoritative_refresh_handles: Vec<String>,
    pub delivery: String,
    pub task_surface: String,
    pub omissions: BTreeMap<String, OmissionReason>,
}
pub(super) fn migrate(text: &str, context: &LegacyContext) -> Result<HandoffEnvelope> {
    let input = text.trim();
    ensure!(
        !input.is_empty() && input.len() <= MAX_LEGACY_BYTES,
        "legacy handoff exceeds the bounded input size"
    );
    ensure!(
        !matches!(input.as_bytes().first(), Some(b'{' | b'[')),
        "canonical JSON is not a legacy handoff"
    );
    let current = current::markdown(input);
    let delta_counts = ["## Lane", "## Delta", "## Next"]
        .map(|heading| schema::exact_heading_count(&current, heading));
    let delta_present = delta_counts.iter().any(|count| *count != 0);
    let has_delta = delta_counts.iter().all(|count| *count == 1); /* all three delta sections once */
    let marker = "Terminal parent handoff:";
    // Historical and quoted text cannot supply current authority to the migration.
    let sanitized = current.replace('\u{1f}', "\u{fffd}");
    let active = super::super::readiness_context::current_text(&sanitized);
    let alternate = "Parent handoff:";
    let alternate_count = active
        .lines()
        .filter(|line| line.trim() == alternate)
        .count();
    let terminal_count = active.matches(marker).count() + alternate_count;
    ensure!(terminal_count <= 1, "legacy terminal boundary is repeated");
    ensure!(
        !(delta_present && terminal_count != 0),
        "mixed legacy handoff boundaries are ambiguous"
    );
    ensure!(
        has_delta || terminal_count == 1,
        "legacy handoff has no recognized canonical boundary"
    );
    if has_delta {
        delta::migrate_delta(&current, context)
    } else {
        let start = active
            .find(marker)
            .or_else(|| active.find(alternate))
            .unwrap_or(0);
        migrate_terminal(&active[start..], context)
    }
}
fn migrate_terminal(line: &str, context: &LegacyContext) -> Result<HandoffEnvelope> {
    let lines = line.lines().collect::<Vec<_>>();
    let table = super::super::child_lifecycle_events::table_record(&lines);
    if let Some((_, consumed)) = &table {
        ensure!(
            lines[*consumed..].iter().all(|line| line.trim().is_empty()),
            "unexpected content after handoff table"
        );
    }
    let line = table.as_ref().map_or(line, |(record, _)| record.as_str());
    let body = line
        .strip_prefix("Terminal parent handoff:")
        .ok_or_else(|| anyhow::anyhow!("invalid terminal handoff boundary"))?;
    let normalized = body.replace('\n', " ");
    let fields = fields(
        &normalized,
        &"event id|issue/pr|child task|parent task|branch|worktree|head|clean/index|last proof|current gate|preserved reservation/artifacts|parent next action|delivery|task surface|message permission"
            .split('|')
            .collect::<Vec<_>>(),
        '=',
    )?;
    let issue_pr = issue_pr_terminal(schema::required(&fields, "issue/pr")?)?;
    schema::require_same(&fields, "branch", &context.branch)?;
    schema::require_same(&fields, "worktree", &context.worktree)?;
    let (dirty, index) = schema::clean_index(schema::required(&fields, "clean/index")?)?;
    ensure!(
        schema::required(&fields, "delivery")? == "confirmed",
        "terminal delivery is not confirmed"
    );
    ensure!(
        schema::required(&fields, "task surface")? == "codex task/thread",
        "terminal task surface is not canonical"
    );
    // An explicit permission state must agree with the claimed delivery; omission remains legacy-compatible.
    ensure!(
        fields
            .get("message permission")
            .is_none_or(|value| value.eq_ignore_ascii_case("granted")),
        "terminal message permission is not granted"
    );
    let child_task = schema::optional_value(&fields, "child task")?;
    let parent_task = schema::optional_value(&fields, "parent task")?;
    // Trusted task identities bind the reported handoff to its intended recipient.
    ensure!(
        child_task
            .as_ref()
            .zip(context.child_task.as_ref())
            .is_some_and(|(actual, expected)| actual == expected),
        "child task authority"
    );
    ensure!(
        parent_task
            .as_ref()
            .zip(context.parent_task.as_ref())
            .is_some_and(|(actual, expected)| actual == expected),
        "parent task authority"
    );
    let preserved_artifacts = schema::optional_value(&fields, "preserved reservation/artifacts")?;
    let last_proof = schema::required(&fields, "last proof")?;
    let mut volatile = volatile_from(
        context,
        issue_pr,
        event_from_id(schema::required(&fields, "event id")?, "", last_proof)?,
    );
    volatile.dirty_index_state = DirtyIndexState { dirty, index };
    volatile.base_head_sha = BaseHeadSha {
        base: context.base.clone(),
        head: schema::required(&fields, "head")?.to_owned(),
    };
    volatile.child_task = child_task;
    volatile.parent_task = parent_task;
    volatile.preserved_artifacts = preserved_artifacts;
    schema::required(&fields, "current gate")?.clone_into(&mut volatile.external_gate);
    schema::required(&fields, "parent next action")?.clone_into(&mut volatile.next_action);
    schema::required(&fields, "delivery")?.clone_into(&mut volatile.delivery);
    schema::required(&fields, "task surface")?.clone_into(&mut volatile.task_surface);
    if !volatile.verification.contains(&last_proof.to_owned()) {
        volatile.verification.push(last_proof.to_owned());
    }
    Ok(HandoffEnvelope::new(context.stable.clone(), volatile))
}
fn volatile_from(
    context: &LegacyContext,
    issue_pr: IssuePrIdentity,
    event: HandoffEvent,
) -> HandoffVolatile {
    HandoffVolatile {
        issue_pr_identity: issue_pr,
        owner_worktree: OwnerWorktree {
            owner: context.owner.clone(),
            branch: context.branch.clone(),
            worktree: context.worktree.clone(),
        },
        base_head_sha: BaseHeadSha {
            base: context.base.clone(),
            head: String::new(),
        },
        dirty_index_state: context.dirty_index_state.clone(),
        checks: context.checks.clone(),
        unresolved_review_threads: context.unresolved_review_threads.clone(),
        selected_reviewer_state: context.selected_reviewer_state.clone(),
        verification: context.verification.clone(),
        active_obligation: context.active_obligation.clone(),
        external_gate: context.external_gate.clone(),
        next_action: String::new(),
        child_task: context.child_task.clone(),
        parent_task: context.parent_task.clone(),
        preserved_artifacts: context.preserved_artifacts.clone(),
        authoritative_refresh_handles: context.authoritative_refresh_handles.clone(),
        delivery: context.delivery.clone(),
        task_surface: context.task_surface.clone(),
        omissions: context.omissions.clone(),
        event,
    }
}
fn event_from_id(id: &str, kind: &str, delta: &str) -> Result<HandoffEvent> {
    let mut parts = id.split('|');
    let (Some(event_kind), Some(lane), Some(subject), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        bail!("legacy event id is malformed");
    };
    ensure!(
        !event_kind.is_empty() && !lane.is_empty() && !subject.is_empty(),
        "legacy event id is malformed"
    );
    ensure!(
        kind.is_empty() || event_kind == kind,
        "legacy event kind conflicts with event id"
    );
    Ok(HandoffEvent {
        id: id.into(),
        kind: event_kind.into(),
        lane: lane.into(),
        subject: subject.into(),
        delta: delta.into(),
    })
}
fn issue_pr(issue: &str, pr: &str) -> Result<IssuePrIdentity> {
    Ok(IssuePrIdentity {
        issue: number_opt(issue, "issue")?,
        pr: number_opt(pr, "PR")?,
    })
}
fn issue_pr_terminal(value: &str) -> Result<IssuePrIdentity> {
    let (issue, pr) = value
        .split_once(" / PR")
        .ok_or_else(|| anyhow::anyhow!("terminal issue/pr is malformed"))?;
    issue_pr(issue, pr.trim())
}
fn number_opt(value: &str, field: &str) -> Result<Option<u64>> {
    let value = value.trim().trim_start_matches('#');
    if matches!(
        value.to_ascii_lowercase().as_str(),
        "not-created" | "not created" | "not-applicable" | "not applicable" | "n/a" | "none"
    ) {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .map(Some)
        .map_err(|_| anyhow::anyhow!("{field} must be numeric or not-created"))
}
