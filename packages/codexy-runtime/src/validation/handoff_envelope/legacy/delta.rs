use super::*;

pub(super) fn migrate_delta(input: &str, context: &LegacyContext) -> Result<HandoffEnvelope> {
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
