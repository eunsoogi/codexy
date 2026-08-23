use super::schema::{
    BaseHeadSha, DirtyIndexState, HandoffEnvelope, HandoffEvent, HandoffVolatile, IssuePrIdentity,
    OwnerWorktree, ReviewThread, StableHandoff,
};
use super::{OmissionReason, schema};
use anyhow::{Result, bail, ensure};
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
    let delta_counts = ["## Lane", "## Delta", "## Next"]
        .map(|heading| schema::exact_heading_count(input, heading));
    let delta_present = delta_counts.iter().any(|count| *count != 0);
    let has_delta = delta_counts.iter().all(|count| *count == 1);
    let marker = "Terminal parent handoff:";
    let terminal_count = input.matches(marker).count();
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
        migrate_delta(input, context)
    } else {
        migrate_terminal(&input[input.find(marker).unwrap_or(0)..], context)
    }
}
fn migrate_delta(input: &str, context: &LegacyContext) -> Result<HandoffEnvelope> {
    let lane = fields(
        schema::section(input, "## Lane")?,
        &[
            "issue", "PR", "branch", "owner", "worktree", "head SHA", "base SHA",
        ],
        ':',
    )?;
    let delta = fields(
        schema::section(input, "## Delta")?,
        &["event id", "event kind", "delta"],
        ':',
    )?;
    let next = fields(
        schema::section(input, "## Next")?,
        &["one next action"],
        ':',
    )?;
    let issue_pr = issue_pr(
        schema::required(&lane, "issue")?,
        schema::required(&lane, "PR")?,
    )?;
    schema::require_same(&lane, "owner", &context.owner)?;
    schema::require_same(&lane, "branch", &context.branch)?;
    schema::require_same(&lane, "worktree", &context.worktree)?;
    let event = event_from_id(
        schema::required(&delta, "event id")?,
        schema::required(&delta, "event kind")?,
        schema::required(&delta, "delta")?,
    )?;
    let mut volatile = volatile_from(context, issue_pr, event);
    volatile.base_head_sha = BaseHeadSha {
        base: schema::required(&lane, "base SHA")?.to_owned(),
        head: schema::required(&lane, "head SHA")?.to_owned(),
    };
    schema::required(&next, "one next action")?.clone_into(&mut volatile.next_action);
    Ok(HandoffEnvelope::new(context.stable.clone(), volatile))
}
fn migrate_terminal(line: &str, context: &LegacyContext) -> Result<HandoffEnvelope> {
    let body = line
        .strip_prefix("Terminal parent handoff:")
        .ok_or_else(|| anyhow::anyhow!("invalid terminal handoff boundary"))?;
    let normalized = body.replace('\n', " ");
    let fields = fields(
        &normalized,
        &"event id|issue/pr|child task|parent task|branch|worktree|head|clean/index|last proof|current gate|preserved reservation/artifacts|parent next action|delivery|task surface"
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
    let child_task = schema::optional_value(&fields, "child task")?;
    let parent_task = schema::optional_value(&fields, "parent task")?;
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
fn fields(input: &str, wanted: &[&str], delimiter: char) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for part in input.split(if delimiter == ':' { '\n' } else { ';' }) {
        let part = part.trim().strip_prefix("- ").unwrap_or(part.trim());
        let Some((key, value)) = part.split_once(delimiter) else {
            continue;
        };
        let key = key.trim();
        if wanted.contains(&key) {
            let value = value.trim();
            ensure!(!value.is_empty(), "legacy field {key} is empty");
            ensure!(
                result.insert(key.to_owned(), value.to_owned()).is_none(),
                "legacy field {key} is repeated"
            );
        }
    }
    Ok(result)
}
