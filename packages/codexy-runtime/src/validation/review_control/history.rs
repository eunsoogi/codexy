use std::collections::HashSet;

use super::policy::Profile;
use serde_json::{Map, Value};

const TERMINAL_RESULTS: [&str; 3] = ["PASS", "BLOCK", "UNOBSERVABLE"];
const REVIEW_KINDS: [&str; 3] = ["full", "delta", "required_current_head"];
const MAX_TERMINAL_REVIEWS: u64 = 3;
pub(super) struct CheckContext<'a> {
    pub(super) expected_reviewer: &'a Value,
    pub(super) reviewed_head: &'a str,
    pub(super) terminal: &'a str,
    pub(super) findings: &'a [Value],
    pub(super) full_count: u64,
    pub(super) delta_count: u64,
    pub(super) terminal_count: u64,
    pub(super) terminal_limit: u64,
    pub(super) profile: &'a Profile,
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
        if event.get("reviewer") != Some(context.expected_reviewer) {
            return Err("review control state terminal review history changes reviewer".into());
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
        check_post_cap(
            post_cap,
            history[1].get("reviewed_head"),
            context.reviewed_head,
        )?;
    } else if required_head_seen != 0 || post_cap.is_some() {
        return Err(
            "review control state post-cap re-review is only valid for the third verdict".into(),
        );
    }
    Ok(())
}
fn check_post_cap(
    value: &Value,
    prior_head: Option<&Value>,
    reviewed_head: &str,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "review control state post_cap_re_review must be an object".to_owned())?;
    reject_unknown(
        object,
        &["reason", "prior_reviewed_head", "qualifying_change"],
        "post_cap_re_review",
    )?;
    let reason = required_text(object, "reason", "post_cap_re_review")?;
    if !matches!(
        reason,
        "mandatory_base_integration" | "in_scope_contract_root_repair"
    ) {
        return Err("review control state post-cap reason is not eligible".into());
    }
    let prior = required_text(object, "prior_reviewed_head", "post_cap_re_review")?;
    if prior == reviewed_head {
        return Err(
            "review control state post-cap re-review must advance the reviewed head".into(),
        );
    }
    if prior_head.and_then(Value::as_str) != Some(prior) {
        return Err("review control state post-cap re-review does not bind the delta head".into());
    }
    let change = object
        .get("qualifying_change")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "review control state post-cap re-review must record qualifying change evidence"
                .to_owned()
        })?;
    reject_unknown(
        change,
        &["from_head", "to_head", "evidence_commit", "finding_ids"],
        "post_cap_re_review.qualifying_change",
    )?;
    if required_text(change, "from_head", "qualifying_change")? != prior
        || required_text(change, "to_head", "qualifying_change")? != reviewed_head
    {
        return Err(
            "review control state qualifying change evidence does not bind the heads".into(),
        );
    }
    let evidence_commit = required_text(change, "evidence_commit", "qualifying_change")?;
    if evidence_commit.len() != 40 || !evidence_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("review control state qualifying change evidence must name a commit".into());
    }
    let finding_ids = change.get("finding_ids");
    if let Some(value) = finding_ids {
        let ids = value.as_array().ok_or_else(|| {
            "review control state qualifying change finding_ids must be an array".to_owned()
        })?;
        let mut unique = HashSet::new();
        for id in ids {
            let id = id.as_str().filter(|id| !id.is_empty()).ok_or_else(|| {
                "review control state qualifying change finding_ids must contain non-empty strings"
                    .to_owned()
            })?;
            if !unique.insert(id) {
                return Err(
                    "review control state qualifying change finding_ids must be unique".into(),
                );
            }
        }
        if reason == "mandatory_base_integration" && !ids.is_empty() {
            return Err(
                "mandatory base integration must not claim contract/root finding ids".into(),
            );
        }
        if reason == "in_scope_contract_root_repair" && ids.is_empty() {
            return Err("contract/root repair must bind at least one finding id".into());
        }
    } else if reason == "in_scope_contract_root_repair" {
        return Err("contract/root repair must bind finding ids".into());
    }
    Ok(())
}

fn required_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("review control state {context} must contain non-empty {key}"))
}

fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(format!(
            "review control state {context} contains an unknown field"
        ));
    }
    Ok(())
}
