use super::child_lane_ownership_phrases::{has_absent_field_value, trimmed_value};

pub(super) fn is_child_delegation_owner_decision(value: &str) -> bool {
    let value = trimmed_value(value);
    is_affirmative_child_owned_value(value)
        || is_affirmative_current_thread_owner_decision(value)
        || (!has_negated_child_routing_requirement(value)
            && has_child_delegation(value)
            && has_routing_only_parent_context(value))
}

pub(super) fn is_affirmative_current_thread_owner_decision(value: &str) -> bool {
    let value = trimmed_value(value);
    matches!(
        parse_owner_assertion(value),
        Some((OwnerSelection::CurrentThreadOwned, _))
    )
}

#[derive(PartialEq)]
enum OwnerSelection {
    CurrentThreadOwned,
}

enum OwnerAssertion {
    Because,
    ImplementationLane,
}

fn parse_owner_assertion(value: &str) -> Option<(OwnerSelection, OwnerAssertion)> {
    let (owner, assertion) = value.split_once(char::is_whitespace)?;
    let selection = parse_owner_selection(owner)?;
    parse_current_thread_assertion(assertion).map(|assertion| (selection, assertion))
}

fn parse_owner_selection(value: &str) -> Option<OwnerSelection> {
    (value == "current-thread-owned").then_some(OwnerSelection::CurrentThreadOwned)
}

fn parse_current_thread_assertion(assertion: &str) -> Option<OwnerAssertion> {
    let assertion = trimmed_value(assertion);
    if assertion
        .strip_prefix("because ")
        .is_some_and(|rationale| !rationale.trim().is_empty())
    {
        return Some(OwnerAssertion::Because);
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
    value.contains("child-owned")
        && !value.contains("not child-owned")
        && !value.starts_with("parent-owned")
        && !has_absent_field_value(value, "child-owned")
}

pub(super) fn is_parent_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    value.starts_with("parent-owned") && !value.contains("not parent-owned")
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
