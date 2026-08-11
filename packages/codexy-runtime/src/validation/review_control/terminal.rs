use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::{
    history::{Blocker, History},
    policy::{self, Profile, Reviewer},
};

const LIFECYCLE_SCHEMA: &str = "codexy.review-terminal-record.v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema: String,
    head_oid: String,
    profile: String,
    reviewer: Option<Reviewer>,
    state: String,
    event_id: String,
    blockers: Vec<Blocker>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleRecord {
    schema: String,
    head_oid: String,
    profile: String,
    reviewer: Option<Reviewer>,
    state: String,
    event_id: String,
    blockers: Vec<Blocker>,
    ledger: History,
}

pub(super) fn check_handoff(plugin_root: &Path, state: &Value) -> Vec<String> {
    match check_state(plugin_root, state) {
        Ok(()) if review_decision_matches(state) => Vec::new(),
        Ok(()) => vec!["profile-routed review decision must bind the terminal disposition".into()],
        Err(error) => vec![error],
    }
}

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
    let Ok(record) = serde_json::from_str::<LifecycleRecord>(record) else {
        return false;
    };
    if record.schema != LIFECYCLE_SCHEMA {
        return false;
    }
    let state = serde_json::json!({
        "headRefOid": record.head_oid,
        "reviewProfile": record.profile,
        "reviewEvidence": {
            "schema": "codexy.review-readiness.v1",
            "head_oid": record.head_oid,
            "profile": record.profile,
            "reviewer": record.reviewer,
            "state": record.state,
            "event_id": record.event_id,
            "blockers": record.blockers,
        },
        "reviewLedger": record.ledger,
    });
    check_state(plugin_root, &state).is_ok()
}

fn check_state(plugin_root: &Path, state: &Value) -> Result<(), String> {
    let profiles =
        policy::load(plugin_root).map_err(|_| "review-profile policy is unavailable".to_owned())?;
    let selected = state
        .get("reviewProfile")
        .and_then(Value::as_str)
        .ok_or_else(|| "profile-routed review selection must be typed and closed".to_owned())?;
    let profile = profiles
        .get(selected)
        .ok_or_else(|| "profile-routed review selection names an unknown profile".to_owned())?;
    let head = state
        .get("headRefOid")
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
        .ok_or_else(|| "profile-routed review evidence must bind the current head".to_owned())?;
    if profile.reviewer.is_none() {
        return (state.get("reviewEvidence").is_none() && state.get("reviewLedger").is_none())
            .then_some(())
            .ok_or_else(|| "light review selection must not attach reviewer evidence".to_owned());
    }
    let evidence = state
        .get("reviewEvidence")
        .cloned()
        .ok_or_else(|| "profile-routed review evidence must be present".to_owned())?;
    let evidence = serde_json::from_value::<Evidence>(evidence)
        .map_err(|_| "profile-routed review evidence must be typed and closed".to_owned())?;
    let history = state.get("reviewLedger").cloned().ok_or_else(|| {
        "profile-routed review evidence must bind a typed terminal ledger event".to_owned()
    })?;
    let history = serde_json::from_value::<History>(history)
        .map_err(|_| "profile-routed review ledger must be typed and closed".to_owned())?;
    terminal_matches(profile, selected, head, &evidence, &history)
        .then_some(())
        .ok_or_else(|| {
            "profile-routed review evidence must bind the selected reviewer and current-head terminal state"
                .to_owned()
        })
}

fn terminal_matches(
    profile: &Profile,
    selected: &str,
    head: &str,
    evidence: &Evidence,
    history: &History,
) -> bool {
    history.validate().is_ok()
        && evidence.schema == "codexy.review-readiness.v1"
        && evidence.profile == selected
        && evidence.head_oid == head
        && evidence.reviewer == profile.reviewer
        && history.events.last().is_some_and(|event| {
            event.id == evidence.event_id
                && event.profile == selected
                && event.head_oid == evidence.head_oid
                && matches!(event.state.as_str(), "passed" | "parent_decision")
                && event.state == evidence.state
                && event.blockers == evidence.blockers
                && (event.state == "parent_decision"
                    || event.blockers.iter().all(|blocker| blocker.resolved))
        })
}

fn review_decision_matches(state: &Value) -> bool {
    let decision = state.get("reviewDecision").and_then(Value::as_str);
    let profile = state.get("reviewProfile").and_then(Value::as_str);
    let terminal_state = state
        .get("reviewEvidence")
        .and_then(|evidence| evidence.get("state"))
        .and_then(Value::as_str);
    match (profile, terminal_state) {
        (Some("light"), None) => decision == Some("NOT_REQUIRED"),
        (Some("standard" | "strict"), Some("passed")) => decision == Some("APPROVED"),
        (Some("standard" | "strict"), Some("parent_decision")) => {
            decision == Some("PARENT_DECISION")
        }
        _ => false,
    }
}
