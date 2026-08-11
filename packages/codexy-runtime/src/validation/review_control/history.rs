use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::{packet::Escalation, policy};

const SCHEMA: &str = "codexy.review-ledger.v1";

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct History {
    pub(super) schema: String,
    pub(super) events: Vec<Event>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Event {
    pub(super) id: String,
    pub(super) predecessor_event_id: Option<String>,
    pub(super) profile: String,
    pub(super) base_oid: String,
    pub(super) head_oid: String,
    pub(super) state: String,
    pub(super) full_used: u8,
    pub(super) delta_used: u8,
    pub(super) blockers: Vec<Blocker>,
    pub(super) boundaries: Vec<String>,
    pub(super) escalation: Option<Escalation>,
}

#[derive(Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Blocker {
    pub(super) id: String,
    pub(super) defect_class: String,
    pub(super) resolved: bool,
    pub(super) reopen_count: u8,
}

impl History {
    pub(super) fn new() -> Self {
        Self {
            schema: SCHEMA.into(),
            events: Vec::new(),
        }
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.schema != SCHEMA {
            bail!("review ledger schema is invalid");
        }
        for (index, event) in self.events.iter().enumerate() {
            if !valid_id(&event.id)
                || !valid_id(&event.base_oid)
                || event.predecessor_event_id.as_deref()
                    != index
                        .checked_sub(1)
                        .and_then(|prior| self.events.get(prior))
                        .map(|prior| prior.id.as_str())
                || !valid_boundaries(&event.boundaries)
                || !valid_blockers(&event.blockers)
            {
                bail!("review ledger event identity or evidence is invalid");
            }
        }
        if self
            .events
            .iter()
            .map(|event| event.id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != self.events.len()
        {
            bail!("review ledger event identities must be unique");
        }
        if !valid_path(&self.events) {
            bail!("review ledger is not one bounded typed review cycle");
        }
        Ok(())
    }
}

fn valid_path(events: &[Event]) -> bool {
    match events {
        [] => true,
        [event] => light_passed(event) || full_review(event) || unavailable(event),
        [first, second] => {
            (full_review(first) && passed_after(first, second, 0))
                || (full_review(first) && delta_after(first, second))
                || escalated_full(first, second)
        }
        [first, second, third] => {
            (full_review(first) && delta_after(first, second) && passed_after(second, third, 1))
                || (full_review(first)
                    && delta_after(first, second)
                    && parent_decision(second, third))
                || (escalated_full(first, second) && passed_after(second, third, 0))
                || (escalated_full(first, second) && delta_after(second, third))
        }
        [unobservable, full, delta, terminal] => {
            escalated_full(unobservable, full)
                && delta_after(full, delta)
                && (passed_after(delta, terminal, 1) || parent_decision(delta, terminal))
        }
        _ => false,
    }
}

fn light_passed(event: &Event) -> bool {
    event.profile == "light"
        && event.state == "passed"
        && event.escalation.is_none()
        && (event.full_used, event.delta_used) == (0, 0)
        && event.blockers.is_empty()
}

fn full_review(event: &Event) -> bool {
    reviewed_profile(&event.profile)
        && event.state == "full"
        && event.escalation.is_none()
        && (event.full_used, event.delta_used) == (1, 0)
}

fn unavailable(event: &Event) -> bool {
    reviewed_profile(&event.profile)
        && event.state == "unobservable"
        && event.escalation.is_none()
        && (event.full_used, event.delta_used) == (0, 0)
}

fn escalated_full(unobservable: &Event, full: &Event) -> bool {
    unavailable(unobservable)
        && full.state == "full"
        && full.head_oid == unobservable.head_oid
        && (full.full_used, full.delta_used) == (1, 0)
        && full.escalation.as_ref().is_some_and(|escalation| {
            escalation.discarded_lower_profile
                && escalation.predecessor_event_id == unobservable.id
                && escalation.from_profile == unobservable.profile
                && policy::is_strictly_higher(&unobservable.profile, &full.profile)
        })
}

fn delta_after(full: &Event, delta: &Event) -> bool {
    delta.state == "delta"
        && delta.profile == full.profile
        && delta.base_oid == full.head_oid
        && delta.head_oid != full.head_oid
        && delta.escalation.is_none()
        && (delta.full_used, delta.delta_used) == (1, 1)
        && preserves_blockers(full, delta)
}

fn passed_after(prior: &Event, passed: &Event, delta_used: u8) -> bool {
    passed.state == "passed"
        && passed.profile == prior.profile
        && passed.head_oid == prior.head_oid
        && passed.escalation.is_none()
        && (passed.full_used, passed.delta_used) == (1, delta_used)
        && passed.blockers.iter().all(|blocker| blocker.resolved)
        && preserves_blockers(prior, passed)
}

fn parent_decision(delta: &Event, decision: &Event) -> bool {
    decision.state == "parent_decision"
        && decision.profile == delta.profile
        && decision.head_oid == delta.head_oid
        && decision.escalation.is_none()
        && (decision.full_used, decision.delta_used) == (1, 1)
        && preserves_blockers(delta, decision)
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

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn reviewed_profile(profile: &str) -> bool {
    matches!(profile, "standard" | "strict")
}

fn valid_boundaries(boundaries: &[String]) -> bool {
    !boundaries.is_empty()
        && boundaries.iter().all(|boundary| !boundary.is_empty())
        && boundaries.iter().collect::<BTreeSet<_>>().len() == boundaries.len()
}

fn valid_blockers(blockers: &[Blocker]) -> bool {
    blockers
        .iter()
        .all(|blocker| valid_id(&blocker.id) && !blocker.defect_class.is_empty())
        && blocker_map(blockers).is_some()
}

fn blocker_map(blockers: &[Blocker]) -> Option<BTreeMap<&str, &Blocker>> {
    let entries = blockers
        .iter()
        .map(|blocker| (blocker.id.as_str(), blocker))
        .collect::<BTreeMap<_, _>>();
    (entries.len() == blockers.len()).then_some(entries)
}
