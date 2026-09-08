use std::collections::HashSet;

use super::policy::Profile;
use serde_json::{Map, Value};

mod fields;
mod final_state;
mod post_cap;

use self::fields::{reject_unknown, required_text};

const TERMINAL_RESULTS: [&str; 3] = ["PASS", "BLOCK", "UNOBSERVABLE"];
const REVIEW_KINDS: [&str; 3] = ["full", "delta", "required_current_head"];
const MAX_TERMINAL_REVIEWS: u64 = 3;
pub(super) struct CheckContext<'a> {
    pub(super) expected_reviewer: &'a Value,
    pub(super) legacy_reviewer: Option<&'a Value>,
    pub(super) legacy_history_boundary: Option<usize>,
    pub(super) legacy_history_event: Option<usize>,
    pub(super) reviewed_head: &'a str,
    pub(super) terminal: &'a str,
    pub(super) findings: &'a [Value],
    pub(super) full_count: u64,
    pub(super) delta_count: u64,
    pub(super) terminal_count: u64,
    pub(super) terminal_limit: u64,
    pub(super) profile: &'a Profile,
    pub(super) current_head: &'a str,
    pub(super) base_oid: Option<&'a str>,
    pub(super) issue_number: u64,
    pub(super) repository: Option<&'a str>,
    pub(super) pull_request: Option<u64>,
    pub(super) pr_state: Option<&'a Value>,
}
pub(super) fn check(
    control: &Map<String, Value>,
    context: &CheckContext<'_>,
) -> Result<(), String> {
    if context.full_count != u64::from(context.profile.full_review_limit)
        || context.delta_count > u64::from(context.profile.delta_recheck_limit)
    {
        return Err("review control state exceeds the bounded review cycle".into());
    }
    if context.terminal_count == 0
        || context.terminal_count > context.terminal_limit
        || context.terminal_limit > MAX_TERMINAL_REVIEWS
    {
        return Err(
            "review control state terminal review count exceeds issue-wide maximum of 3".into(),
        );
    }
    let history = control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .ok_or_else(|| "review control state must carry terminal review history".to_owned())?;
    if history.len() as u64 != context.terminal_count {
        return Err("review control state terminal review history is truncated".into());
    }
    let native_provenance = control.contains_key("native_history_provenance");
    if context.legacy_reviewer.is_some()
        != (context.legacy_history_boundary.is_some() || context.legacy_history_event.is_some())
    {
        return Err("review control state reviewer migration is incomplete".into());
    }
    if context.legacy_history_boundary.is_some() && context.legacy_history_event.is_some() {
        return Err("review control state reviewer migration has conflicting directions".into());
    }
    if let Some(boundary) = context.legacy_history_boundary
        && (boundary == 0 || boundary > history.len())
    {
        return Err("review control state reviewer migration boundary is invalid".into());
    }
    if let Some(index) = context.legacy_history_event
        && (index == 0 || index >= history.len())
    {
        return Err("review control state historical reviewer exception is invalid".into());
    }
    let mut ids = HashSet::new();
    let mut full_seen = 0;
    let mut delta_seen = 0;
    let mut required_head_seen = 0;
    let mut heads = HashSet::new();
    for (index, event) in history.iter().enumerate() {
        let event = event.as_object().ok_or_else(|| {
            "review control state terminal review history entry is not an object".to_owned()
        })?;
        reject_unknown(
            event,
            &[
                "id",
                "kind",
                "reviewer",
                "policy_reviewer",
                "source_reviewer",
                "reviewed_head",
                "terminal_result",
                "unresolved_findings",
            ],
            "terminal review history entry",
        )?;
        let id = required_text(event, "id", "terminal review history entry")?;
        if !ids.insert(id.to_owned()) {
            return Err(
                "review control state terminal review history has duplicate review id".into(),
            );
        }
        let kind = required_text(event, "kind", "terminal review history entry")?;
        if REVIEW_KINDS.get(index) != Some(&kind) {
            return Err("review control state terminal review history is reordered".into());
        }
        match kind {
            "full" => full_seen += 1,
            "delta" => delta_seen += 1,
            "required_current_head" => required_head_seen += 1,
            _ => return Err("review control state terminal review kind is invalid".into()),
        }
        if context.legacy_history_event == Some(index) && kind != "delta" {
            return Err(
                "review control state historical reviewer exception must bind the delta".into(),
            );
        }
        let expected_reviewer = if context.legacy_history_event == Some(index) {
            context.legacy_reviewer.ok_or_else(|| {
                "review control state historical reviewer exception is incomplete".to_owned()
            })?
        } else {
            match context.legacy_history_boundary {
                Some(boundary) if index < boundary => context.legacy_reviewer.ok_or_else(|| {
                    "review control state reviewer migration is incomplete".to_owned()
                })?,
                _ => context.expected_reviewer,
            }
        };
        let policy_reviewer = event
            .get("policy_reviewer")
            .or_else(|| event.get("reviewer"));
        if event.contains_key("policy_reviewer") != event.contains_key("source_reviewer") {
            return Err(
                "review control state recovered history reviewer provenance is incomplete".into(),
            );
        }
        if event.contains_key("policy_reviewer") && !native_provenance {
            return Err("review control state recovered history lacks native provenance".into());
        }
        if policy_reviewer != Some(expected_reviewer) {
            return Err("review control state terminal review history changes reviewer".into());
        }
        if event.contains_key("policy_reviewer")
            && event.get("source_reviewer") != event.get("reviewer")
        {
            return Err(
                "review control state recovered history changes source reviewer identity".into(),
            );
        }
        let event_head = required_text(event, "reviewed_head", "terminal review history entry")?;
        if !heads.insert(event_head) {
            return Err(
                "review control state terminal review history has duplicate reviewed head".into(),
            );
        }
        let event_terminal =
            required_text(event, "terminal_result", "terminal review history entry")?;
        if !TERMINAL_RESULTS.contains(&event_terminal) {
            return Err(
                "review control state terminal review history has an invalid result".into(),
            );
        }
        if event
            .get("unresolved_findings")
            .and_then(Value::as_array)
            .is_none()
        {
            return Err("review control state terminal review history must list findings".into());
        }
        if index + 1 == history.len()
            && (event_head != context.reviewed_head
                || event_terminal != context.terminal
                || event.get("unresolved_findings")
                    != Some(&Value::Array(context.findings.to_owned())))
        {
            return Err("review control state projection disagrees with terminal history".into());
        }
    }
    if full_seen != context.full_count || delta_seen != context.delta_count {
        return Err("review control state review counters disagree with terminal history".into());
    }
    let post_cap = control.get("post_cap_re_review");
    if context.terminal_count == MAX_TERMINAL_REVIEWS {
        if context.profile.post_cap_re_review_limit != 1
            || required_head_seen != 1
            || context.full_count != 1
            || context.delta_count != 1
        {
            return Err(
                "review control state third verdict requires full and delta history".into(),
            );
        }
        let post_cap = post_cap.ok_or_else(|| {
            "review control state third terminal verdict requires post_cap_re_review".to_owned()
        })?;
        post_cap::check(
            post_cap,
            history[1].get("reviewed_head"),
            history
                .last()
                .and_then(Value::as_object)
                .and_then(|event| event.get("reviewed_head"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    "review control state third event has no reviewed head".to_owned()
                })?,
        )?;
    } else if required_head_seen != 0 || post_cap.is_some() {
        return Err(
            "review control state post-cap re-review is only valid for the third verdict".into(),
        );
    }
    final_state::check(
        control.get("final_disposition"),
        history,
        context.reviewed_head,
        context.terminal,
        context.findings,
        context.current_head,
        context.base_oid,
        context.issue_number,
        context.repository,
        context.pull_request,
        context.pr_state,
    )?;
    Ok(())
}
