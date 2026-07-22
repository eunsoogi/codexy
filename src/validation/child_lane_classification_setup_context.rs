use super::child_lane_classification_authority::{
    AuthorityRecordAction, lane_authority_record_state_before,
};
use super::child_lane_classification_boundaries::{
    current_lane_start, is_inside_task_classification,
};
use super::child_lane_classification_control::normalize_metadata_prefix;
use super::child_lane_classification_setup::{
    has_complete_gfm_display_before, latest_classification_before,
};
use super::child_lane_owner_decision::{
    LaneOwnershipMetadata, OwnerSelection, is_child_delegation_owner_decision,
    is_parent_owned_value, parse_lane_ownership_metadata,
};
use super::child_lane_ownership_phrases::{
    field_value, has_absent_field_value, metadata_key, trimmed_value,
};

pub(super) fn child_setup_context_applies(
    lines: &[&str],
    setup_index: usize,
    explicit_child_scope: bool,
) -> bool {
    let classification_complete = latest_classification_before(lines, setup_index)
        .is_some_and(|snapshot| snapshot.has_complete_authority_record());
    if let Some(applies) = lane_authority_record_state_before(lines, setup_index)
        .validation_applies(
            classification_complete,
            AuthorityRecordAction::Setup {
                explicit_child_scope,
            },
        )
    {
        return applies;
    }
    for (index, line) in lines
        .iter()
        .enumerate()
        .take(setup_index + 1)
        .rev()
        .map(|(index, line)| (index, trimmed_value(line)))
    {
        if index != setup_index && is_lane_context_boundary(lines, index, line) {
            return requires_child_setup_validation(line)
                || (explicit_child_scope && has_authoritative_non_child_owner(line))
                || has_complete_child_classification_before(lines, setup_index);
        }
        if matches!(
            parse_lane_ownership_metadata(line),
            LaneOwnershipMetadata::Invalid
        ) {
            return true;
        }
        if is_parent_owned_lane_evidence(line) {
            return explicit_child_scope;
        }
        if has_authoritative_external_owner(line) {
            return explicit_child_scope;
        }
        if is_child_owned_lane_evidence(line) {
            return true;
        }
    }
    lines
        .iter()
        .enumerate()
        .skip(setup_index + 1)
        .map(|(index, line)| (index, trimmed_value(line)))
        .take_while(|(index, line)| !is_later_lane_boundary(lines, *index, line))
        .any(|(_, line)| is_child_owned_lane_evidence(line))
        || has_complete_child_classification_before(lines, setup_index)
}

pub(super) fn prior_child_lane_context_applies(lines: &[&str], index: usize) -> bool {
    let classification_complete = latest_classification_before(lines, index)
        .is_some_and(|snapshot| snapshot.has_complete_authority_record());
    if let Some(applies) = lane_authority_record_state_before(lines, index)
        .validation_applies(classification_complete, AuthorityRecordAction::Control)
    {
        return applies;
    }
    for (candidate_index, line) in
        lines
            .iter()
            .enumerate()
            .take(index + 1)
            .rev()
            .map(|(candidate_index, line)| {
                (
                    candidate_index,
                    trimmed_value(normalize_metadata_prefix(line)),
                )
            })
    {
        if candidate_index != index && is_lane_context_boundary(lines, candidate_index, line) {
            return requires_child_setup_validation(line)
                || has_complete_child_classification_before(lines, index);
        }
        if matches!(
            parse_lane_ownership_metadata(line),
            LaneOwnershipMetadata::Invalid
        ) {
            return true;
        }
        if is_parent_owned_lane_evidence(line) {
            return false;
        }
        if is_child_owned_lane_evidence(line) {
            return true;
        }
    }
    has_complete_child_classification_before(lines, index)
}

fn is_child_owned_lane_evidence(line: &str) -> bool {
    let gfm_display_row = line.trim_start().starts_with('|');
    let line = metadata_key(line);
    matches!(line, "child-owned" | "child-owned lane")
        || has_present_child_owner_metadata(line)
        || (!gfm_display_row
            && field_value(line, "owner decision").is_some_and(is_child_delegation_owner_decision))
        || matches!(
            parse_lane_ownership_metadata(line),
            LaneOwnershipMetadata::Valid(
                OwnerSelection::ChildOwned | OwnerSelection::CurrentThreadOwned
            )
        )
        || ["owner", "lane owner"].into_iter().any(|field| {
            field_value(line, field).is_some_and(|value| {
                matches!(trimmed_value(value), "child-owned" | "current-thread-owned")
            })
        })
}

fn requires_child_setup_validation(line: &str) -> bool {
    matches!(
        parse_lane_ownership_metadata(line),
        LaneOwnershipMetadata::Invalid
            | LaneOwnershipMetadata::Valid(
                OwnerSelection::ChildOwned | OwnerSelection::CurrentThreadOwned
            )
    ) || is_child_owned_lane_evidence(line)
}

fn has_complete_child_classification_before(lines: &[&str], end: usize) -> bool {
    latest_classification_before(lines, end)
        .is_some_and(|snapshot| snapshot.has_complete_child_display())
        || {
            let lane_start = current_lane_start(lines, end);
            has_complete_gfm_display_before(&lines[lane_start..end], end - lane_start)
        }
}

fn has_present_child_owner_metadata(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, value)| {
        metadata_key(key) == "child owner"
            && !trimmed_value(value).is_empty()
            && !has_absent_field_value(value, "child owner")
    })
}

fn is_parent_owned_lane_evidence(line: &str) -> bool {
    let gfm_display_row = line.trim_start().starts_with('|');
    let line = metadata_key(line);
    if !gfm_display_row
        && field_value(line, "owner decision").is_some_and(|value| {
            is_parent_owned_value(value) && !is_child_delegation_owner_decision(value)
        })
    {
        return true;
    }
    matches!(
        parse_lane_ownership_metadata(line),
        LaneOwnershipMetadata::Valid(OwnerSelection::ParentOwned)
    ) || ["owner", "lane owner"].into_iter().any(|field| {
        field_value(line, field).is_some_and(|value| trimmed_value(value) == "parent-owned")
    })
}

fn has_authoritative_non_child_owner(line: &str) -> bool {
    is_parent_owned_lane_evidence(line) || has_authoritative_external_owner(line)
}

fn has_authoritative_external_owner(line: &str) -> bool {
    matches!(
        parse_lane_ownership_metadata(metadata_key(line)),
        LaneOwnershipMetadata::Valid(OwnerSelection::ExternalHumanOwned)
    )
}

fn is_later_lane_boundary(lines: &[&str], index: usize, line: &str) -> bool {
    let line = metadata_key(line);
    is_parent_owned_lane_evidence(line)
        || has_authoritative_external_owner(line)
        || "pr:|pull request:|review response:|maintainer reassignment:"
            .split('|')
            .any(|marker| line.starts_with(marker))
        || line.starts_with("lane ownership:")
        || (is_owner_metadata(line) && !is_inside_task_classification(lines, index))
}

fn is_lane_context_boundary(lines: &[&str], index: usize, line: &str) -> bool {
    let line = metadata_key(line);
    "pr:|pull request:|review response:|maintainer reassignment:"
        .split('|')
        .any(|marker| line.starts_with(marker))
        || line.starts_with("lane ownership:")
        || (is_owner_metadata(line) && !is_inside_task_classification(lines, index))
}

fn is_owner_metadata(line: &str) -> bool {
    "owner:|child owner:|lane owner:|owner decision:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}
