use super::child_lane_ownership_phrases::{has_absent_field_value, metadata_key, trimmed_value};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OwnerSelection {
    ParentOwned,
    ChildOwned,
    CurrentThreadOwned,
    ExternalHumanOwned,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LaneOwnershipMetadata {
    Absent,
    Invalid,
    Valid(OwnerSelection),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OwnerAffirmation {
    Affirmative,
    Denied,
}

struct OwnerDecision {
    selection: OwnerSelection,
    affirmation: OwnerAffirmation,
}

pub(super) fn is_child_delegation_owner_decision(value: &str) -> bool {
    let value = trimmed_value(value);
    is_affirmative_child_owner_decision(value)
        || matches!(
            owner_prefix(value),
            Some(OwnerSelection::ChildOwned | OwnerSelection::CurrentThreadOwned)
        ) && (value.contains("child implementation lane")
            || value.contains("implementation lane"))
        || (!has_negated_child_routing_requirement(value)
            && has_child_delegation(value)
            && has_routing_only_parent_context(value))
}

pub(super) fn is_affirmative_owner_decision_for(value: &str, authority: OwnerSelection) -> bool {
    matches!(
        parse_explicit_owner_decision(trimmed_value(value)),
        Some(OwnerDecision {
            selection,
            affirmation: OwnerAffirmation::Affirmative,
        }) if selection == authority
    )
}

pub(super) fn is_affirmative_child_owner_decision(value: &str) -> bool {
    matches!(
        parse_owner_decision(value),
        Some(OwnerDecision {
            selection: OwnerSelection::ChildOwned | OwnerSelection::CurrentThreadOwned,
            affirmation: OwnerAffirmation::Affirmative,
        })
    )
}

pub(super) fn parse_lane_ownership_metadata(line: &str) -> LaneOwnershipMetadata {
    let Some((key, value)) = line.split_once(':') else {
        return LaneOwnershipMetadata::Absent;
    };
    if metadata_key(key) != "lane ownership" {
        return LaneOwnershipMetadata::Absent;
    }
    parse_owner_selection(value)
        .map_or(LaneOwnershipMetadata::Invalid, LaneOwnershipMetadata::Valid)
}

/// Parses the complete normalized authoritative metadata value, never an owner prefix.
pub(super) fn parse_owner_selection(value: &str) -> Option<OwnerSelection> {
    match trimmed_value(value) {
        "parent-owned" => Some(OwnerSelection::ParentOwned),
        "child-owned" => Some(OwnerSelection::ChildOwned),
        "current-thread-owned" => Some(OwnerSelection::CurrentThreadOwned),
        "external/human-owned" => Some(OwnerSelection::ExternalHumanOwned),
        _ => None,
    }
}

fn parse_owner_decision(value: &str) -> Option<OwnerDecision> {
    let value = trimmed_value(value);
    parse_explicit_owner_decision(value).or_else(|| parse_legacy_owner_decision(value))
}

fn parse_explicit_owner_decision(value: &str) -> Option<OwnerDecision> {
    let (affirmation, value) = value.split_once(char::is_whitespace)?;
    let affirmation = match affirmation {
        "affirmative" => OwnerAffirmation::Affirmative,
        "denied" => OwnerAffirmation::Denied,
        _ => return None,
    };
    let (selection, rationale) = value
        .split_once(char::is_whitespace)
        .map_or((value, None), |(selection, remainder)| {
            (selection, Some(remainder))
        });
    if rationale.is_some_and(|rationale| {
        !rationale
            .strip_prefix("because ")
            .is_some_and(|text| !text.trim().is_empty())
    }) {
        return None;
    }
    Some(OwnerDecision {
        selection: parse_owner_selection(selection)?,
        affirmation,
    })
}

fn parse_legacy_owner_decision(value: &str) -> Option<OwnerDecision> {
    let (selection, assertion) = value.split_once(char::is_whitespace)?;
    let remainder = assertion.strip_prefix("implementation lane")?;
    if !remainder.is_empty() && !remainder.starts_with(char::is_whitespace) {
        return None;
    }
    Some(OwnerDecision {
        selection: parse_owner_selection(selection)?,
        affirmation: OwnerAffirmation::Affirmative,
    })
}

fn owner_prefix(value: &str) -> Option<OwnerSelection> {
    let value = trimmed_value(value);
    let selection = value
        .split_once(char::is_whitespace)
        .map_or(value, |(owner, _)| owner);
    parse_owner_selection(selection)
}

pub(super) fn is_affirmative_child_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    owner_prefix(value) == Some(OwnerSelection::ChildOwned)
        && !value.contains("not child-owned")
        && !has_absent_field_value(value, "child-owned")
}

pub(super) fn is_parent_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    owner_prefix(value) == Some(OwnerSelection::ParentOwned) && !value.contains("not parent-owned")
}

fn has_child_delegation(value: &str) -> bool {
    (value.contains("child delegation")
        || value.contains("child-lane setup")
        || value.contains("child routing")
        || value.contains("child thread/worktree owner")
        || value.contains("thread/worktree tool discovery")
        || value.contains("thread tool discovery")
        || value.contains("worktree tool discovery"))
        && !value.contains("without child delegation")
}

fn has_routing_only_parent_context(value: &str) -> bool {
    value.contains("routing-only")
        || value.contains("coordination-only")
        || value.contains("delegation-only")
        || value.contains("child routing required")
        || value.contains("owner required")
        || value.contains("tool discovery only")
        || value.contains("tool-discovery-only")
}

fn has_negated_child_routing_requirement(value: &str) -> bool {
    let value = value.replace("child-routing", "child routing");
    [
        "no child routing required",
        "child routing not required",
        "no child routing is required",
        "child routing is not required",
        "without child routing",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}
