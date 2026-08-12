use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::{
    history::{Blocker, History},
    policy::{self, Profile, Reviewer},
    presence::OptionalField,
};

const LIFECYCLE_SCHEMA: &str = "codexy.review-terminal-record.v1";
const CONTROL_SCHEMA: &str = "codexy.review-control-state.v1";

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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    schema: String,
    profile: String,
    decision: String,
    #[serde(default)]
    evidence: OptionalField<Value>,
    #[serde(default)]
    ledger: OptionalField<Value>,
}

pub(super) fn check_handoff(plugin_root: &Path, state: &Value) -> Vec<String> {
    check_state(plugin_root, state).err().into_iter().collect()
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
        "reviewControl": {
            "schema": CONTROL_SCHEMA,
            "profile": record.profile,
            "decision": if record.state == "parent_decision" { "PARENT_DECISION" } else { "APPROVED" },
            "evidence": {
                "schema": "codexy.review-readiness.v1", "head_oid": record.head_oid,
                "profile": record.profile, "reviewer": record.reviewer, "state": record.state,
                "event_id": record.event_id, "blockers": record.blockers,
            },
            "ledger": record.ledger,
        },
    });
    check_state(plugin_root, &state).is_ok()
}

fn check_state(plugin_root: &Path, state: &Value) -> Result<(), String> {
    let profiles =
        policy::load(plugin_root).map_err(|_| "review-profile policy is unavailable".to_owned())?;
    let control = state.get("reviewControl").cloned().ok_or_else(|| {
        "profile-routed review control state must be typed and namespaced".to_owned()
    })?;
    if control.get("decision").and_then(Value::as_str).is_none() {
        return Err("profile-routed review decision must be typed and closed".into());
    }
    let control = serde_json::from_value::<Control>(control)
        .map_err(|_| "profile-routed review control state must be typed and closed".to_owned())?;
    if control.schema != CONTROL_SCHEMA {
        return Err("profile-routed review control state has an unsupported schema".into());
    }
    let selected = control.profile.as_str();
    let profile = profiles
        .get(selected)
        .ok_or_else(|| "profile-routed review selection names an unknown profile".to_owned())?;
    let head = state
        .get("headRefOid")
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
        .ok_or_else(|| "profile-routed review evidence must bind the current head".to_owned())?;
    if profile.reviewer.is_none() {
        return (control.decision == "NOT_REQUIRED"
            && control.evidence.is_absent()
            && control.ledger.is_absent())
        .then_some(())
        .ok_or_else(|| "light review selection must not attach reviewer evidence".to_owned());
    }
    let evidence = control
        .evidence
        .into_present()
        .ok_or_else(|| "profile-routed review evidence must be present".to_owned())?;
    let evidence = serde_json::from_value::<Evidence>(evidence)
        .map_err(|_| "profile-routed review evidence must be typed and closed".to_owned())?;
    let history = control.ledger.into_present().ok_or_else(|| {
        "profile-routed review evidence must bind a typed terminal ledger event".to_owned()
    })?;
    let history = serde_json::from_value::<History>(history)
        .map_err(|_| "profile-routed review ledger must be typed and closed".to_owned())?;
    if !terminal_matches(profile, selected, head, &evidence, &history) {
        return Err(
            "profile-routed review evidence must bind the selected reviewer and current-head terminal state".into(),
        );
    }
    decision_matches(&control.decision, &evidence.state)
        .then_some(())
        .ok_or_else(|| "profile-routed review decision must bind the terminal disposition".into())
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

fn decision_matches(decision: &str, terminal_state: &str) -> bool {
    matches!(
        (decision, terminal_state),
        ("APPROVED", "passed") | ("PARENT_DECISION", "parent_decision")
    )
}
