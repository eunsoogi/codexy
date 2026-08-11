use std::{collections::BTreeMap, path::Path};

use serde::Deserialize;
use serde_json::Value;

use super::{
    packet::Escalation,
    policy::{self, Reviewer},
};

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
#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Blocker {
    id: String,
    defect_class: String,
    resolved: bool,
    reopen_count: u8,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema: String,
    events: Vec<Event>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    id: String,
    predecessor_event_id: Option<String>,
    profile: String,
    head_oid: String,
    state: String,
    full_used: u8,
    delta_used: u8,
    blockers: Vec<Blocker>,
    boundaries: Vec<String>,
    escalation: Option<Escalation>,
}

pub(super) fn check(plugin_root: &Path, pr_state: &Value) -> Vec<String> {
    let Ok(profiles) = policy::load(plugin_root) else {
        return vec!["review-profile policy is unavailable".into()];
    };
    let Some(selected) = pr_state.get("reviewProfile").and_then(Value::as_str) else {
        return vec!["profile-routed review selection must be typed and closed".into()];
    };
    let Some(profile) = profiles.get(selected) else {
        return vec!["profile-routed review selection names an unknown profile".into()];
    };
    let Some(raw) = pr_state.get("reviewEvidence") else {
        return profile
            .reviewer
            .is_none()
            .then(Vec::new)
            .unwrap_or_else(|| vec!["profile-routed review evidence must be present".into()]);
    };
    if profile.reviewer.is_none() {
        return vec!["light review selection must not attach reviewer evidence".into()];
    }
    let Ok(evidence) = serde_json::from_value::<Evidence>(raw.clone()) else {
        return vec!["profile-routed review evidence must be typed and closed".into()];
    };
    let Some(raw_ledger) = pr_state.get("reviewLedger") else {
        return vec![
            "profile-routed review evidence must bind a typed terminal ledger event".into(),
        ];
    };
    let Ok(ledger) = serde_json::from_value::<Ledger>(raw_ledger.clone()) else {
        return vec!["profile-routed review ledger must be typed and closed".into()];
    };
    if evidence.schema != "codexy.review-readiness.v1"
        || evidence.profile != selected
        || pr_state.get("headRefOid").and_then(Value::as_str) != Some(&evidence.head_oid)
        || evidence.reviewer != profile.reviewer
        || evidence.state != "passed"
        || !terminal_matches(&ledger, &evidence, selected)
    {
        return vec![
            "profile-routed review evidence must bind the selected reviewer and current head PASS"
                .into(),
        ];
    }
    Vec::new()
}

fn terminal_matches(ledger: &Ledger, evidence: &Evidence, selected: &str) -> bool {
    if ledger.schema != "codexy.review-ledger.v1" || ledger.events.is_empty() {
        return false;
    }
    if ledger.events.iter().enumerate().any(|(index, event)| {
        event.predecessor_event_id.as_deref()
            != index
                .checked_sub(1)
                .and_then(|prior| ledger.events.get(prior))
                .map(|event| event.id.as_str())
    }) || !valid_cycle(&ledger.events)
    {
        return false;
    }
    let Some(event) = ledger.events.last() else {
        return false;
    };
    event.id == evidence.event_id
        && event.profile == selected
        && event.head_oid == evidence.head_oid
        && event.state == "passed"
        && event.blockers == evidence.blockers
        && event.blockers.iter().all(|blocker| blocker.resolved)
}

fn valid_cycle(events: &[Event]) -> bool {
    if events
        .iter()
        .any(|event| event.boundaries.is_empty() || event.boundaries.iter().any(String::is_empty))
    {
        return false;
    }
    match events {
        [full, passed] => {
            full_review(full) && terminal_after(full, passed, 0) && preserves_blockers(full, passed)
        }
        [full, delta, passed] if full.state == "full" && delta.state == "delta" => {
            full_review(full)
                && delta.profile == full.profile
                && delta.head_oid != full.head_oid
                && delta.escalation.is_none()
                && (delta.full_used, delta.delta_used) == (1, 1)
                && preserves_blockers(full, delta)
                && terminal_after(delta, passed, 1)
                && preserves_blockers(delta, passed)
        }
        [unobservable, full, passed]
            if unobservable.state == "unobservable" && full.state == "full" =>
        {
            unobservable.escalation.is_none()
                && (unobservable.full_used, unobservable.delta_used) == (0, 0)
                && full.head_oid == unobservable.head_oid
                && full.escalation.as_ref().is_some_and(|escalation| {
                    escalation.discarded_lower_profile
                        && escalation.predecessor_event_id == unobservable.id
                        && escalation.from_profile == unobservable.profile
                        && policy::is_strictly_higher(&unobservable.profile, &full.profile)
                })
                && (full.full_used, full.delta_used) == (1, 0)
                && terminal_after(full, passed, 0)
                && preserves_blockers(full, passed)
        }
        _ => false,
    }
}

fn full_review(event: &Event) -> bool {
    event.state == "full"
        && event.escalation.is_none()
        && (event.full_used, event.delta_used) == (1, 0)
}

fn terminal_after(prior: &Event, terminal: &Event, delta_used: u8) -> bool {
    terminal.state == "passed"
        && terminal.profile == prior.profile
        && terminal.head_oid == prior.head_oid
        && terminal.escalation.is_none()
        && (terminal.full_used, terminal.delta_used) == (1, delta_used)
}

fn preserves_blockers(prior: &Event, next: &Event) -> bool {
    let Some(prior) = blocker_map(&prior.blockers) else {
        return false;
    };
    let Some(next) = blocker_map(&next.blockers) else {
        return false;
    };
    prior.iter().all(|(id, old)| {
        next.get(id).is_some_and(|new| {
            new.defect_class == old.defect_class
                && new.reopen_count
                    == if old.resolved && !new.resolved {
                        old.reopen_count.saturating_add(1)
                    } else {
                        old.reopen_count
                    }
        })
    }) && next.values().all(|new| {
        prior.contains_key(&new.id.as_str())
            || (!new.resolved
                && !prior
                    .values()
                    .any(|old| old.defect_class == new.defect_class))
    })
}

fn blocker_map(blockers: &[Blocker]) -> Option<BTreeMap<&str, &Blocker>> {
    let entries = blockers
        .iter()
        .map(|blocker| (blocker.id.as_str(), blocker))
        .collect::<BTreeMap<_, _>>();
    (entries.len() == blockers.len()).then_some(entries)
}
