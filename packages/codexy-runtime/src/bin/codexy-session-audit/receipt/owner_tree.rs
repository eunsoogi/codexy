use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::{
    super::audit_math::checked_add,
    schema::{Family, OwnerSession, Totals},
    validate_digest,
};

pub(super) fn aggregate_sessions(sessions: &[OwnerSession], owner: &str) -> Result<Totals> {
    if sessions.is_empty() {
        bail!("owner tree must contain at least one session");
    }
    let mut ids = BTreeSet::new();
    let mut totals = Totals::default();
    for session in sessions {
        validate_digest(&session.input_sha256)?;
        if session.owner_root_thread_id != owner {
            bail!("owner-tree session does not match owner boundary");
        }
        if !ids.insert(&session.session_id) {
            bail!("owner tree contains a duplicate session");
        }
        totals.session_count = checked_add(totals.session_count, 1, "owner-tree session count")?;
        add(
            &mut totals.records_observed,
            session.records_observed,
            "records",
        )?;
        add(&mut totals.turn_events, session.turn_events, "turns")?;
        add(
            &mut totals.cumulative_tokens,
            session.cumulative_tokens,
            "tokens",
        )?;
        add(
            &mut totals.tool_input_bytes,
            session.tool_input_bytes,
            "tool input",
        )?;
        add(
            &mut totals.tool_output_bytes,
            session.tool_output_bytes,
            "tool output",
        )?;
        add_family(&mut totals.exec_family, &session.exec_family)?;
        add_family(&mut totals.wait_family, &session.wait_family)?;
    }
    Ok(totals)
}

fn add(target: &mut u64, value: u64, label: &str) -> Result<()> {
    *target = checked_add(*target, value, &format!("owner-tree {label}"))?;
    Ok(())
}

fn add_family(target: &mut Family, value: &Family) -> Result<()> {
    add(&mut target.calls, value.calls, "family calls")?;
    add(&mut target.input_bytes, value.input_bytes, "family input")?;
    add(
        &mut target.output_bytes,
        value.output_bytes,
        "family output",
    )
}
