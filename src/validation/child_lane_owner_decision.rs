use super::child_lane_ownership_phrases::{has_absent_field_value, trimmed_value};

pub(super) fn is_child_delegation_owner_decision(value: &str) -> bool {
    let value = trimmed_value(value);
    is_affirmative_child_owner_decision(value)
        || (!has_negated_child_routing_requirement(value)
            && has_child_delegation(value)
            && has_routing_only_parent_context(value))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OwnerSelection {
    ParentOwned,
    ChildOwned,
    CurrentThreadOwned,
    ExternalHumanOwned,
}

struct OwnerDecision {
    selection: OwnerSelection,
}

enum OwnerAssertion {
    Because,
    ImplementationLane,
}

pub(super) fn is_affirmative_owner_decision_for(value: &str, authority: OwnerSelection) -> bool {
    parse_owner_selection(value) == Some(authority)
        && parse_affirmative_owner_decision(value).is_some()
}

pub(super) fn is_affirmative_child_owner_decision(value: &str) -> bool {
    matches!(
        parse_affirmative_owner_decision(value),
        Some(OwnerDecision {
            selection: OwnerSelection::ChildOwned | OwnerSelection::CurrentThreadOwned
        })
    )
}

pub(super) fn parse_owner_selection(value: &str) -> Option<OwnerSelection> {
    let owner = trimmed_value(value)
        .split_once(char::is_whitespace)
        .map_or(trimmed_value(value), |(owner, _)| owner);
    match owner {
        "parent-owned" => Some(OwnerSelection::ParentOwned),
        "child-owned" => Some(OwnerSelection::ChildOwned),
        "current-thread-owned" => Some(OwnerSelection::CurrentThreadOwned),
        "external/human-owned" => Some(OwnerSelection::ExternalHumanOwned),
        _ => None,
    }
}

fn parse_affirmative_owner_decision(value: &str) -> Option<OwnerDecision> {
    let (owner, assertion) = trimmed_value(value).split_once(char::is_whitespace)?;
    let selection = parse_owner_selection(owner)?;
    parse_owner_assertion(selection, assertion).map(|_| OwnerDecision { selection })
}

fn parse_owner_assertion(selection: OwnerSelection, assertion: &str) -> Option<OwnerAssertion> {
    let assertion = trimmed_value(assertion);
    if assertion
        .strip_prefix("because ")
        .is_some_and(|rationale| !rationale.trim().is_empty())
    {
        return Some(OwnerAssertion::Because);
    }
    if !matches!(
        selection,
        OwnerSelection::ChildOwned | OwnerSelection::CurrentThreadOwned
    ) {
        return None;
    }
    matches!(
        assertion,
        "child implementation lane" | "implementation lane"
    )
    .then_some(OwnerAssertion::ImplementationLane)
    .or_else(|| {
        assertion
            .strip_prefix("implementation lane for ")
            .filter(|rationale| !rationale.trim().is_empty())
            .map(|_| OwnerAssertion::ImplementationLane)
    })
}

pub(super) fn is_affirmative_child_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    parse_owner_selection(value) == Some(OwnerSelection::ChildOwned)
        && !value.contains("not child-owned")
        && !has_absent_field_value(value, "child-owned")
}

pub(super) fn is_parent_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    parse_owner_selection(value) == Some(OwnerSelection::ParentOwned)
        && !value.contains("not parent-owned")
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
