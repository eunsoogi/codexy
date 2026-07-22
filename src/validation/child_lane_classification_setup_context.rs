use super::child_lane_classification_control::normalize_metadata_prefix;
use super::child_lane_classification_fields::{
    ClassificationFields, classification_table_row, is_table_separator,
};
use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};
use super::child_lane_ownership_phrases::{
    field_value, has_absent_field_value, metadata_key, trimmed_value,
};

pub(super) fn child_lane_context_applies(lines: &[&str], setup_index: usize) -> bool {
    for (index, line) in lines
        .iter()
        .enumerate()
        .take(setup_index + 1)
        .rev()
        .map(|(index, line)| (index, trimmed_value(line)))
    {
        if index != setup_index && is_lane_context_boundary(lines, index, line) {
            return is_child_owned_lane_evidence(line)
                || has_complete_child_classification_before(lines, setup_index);
        }
        if is_parent_owned_lane_evidence(line) {
            return false;
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
            return is_child_owned_lane_evidence(line)
                || has_complete_child_classification_before(lines, index);
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
    let line = metadata_key(line);
    matches!(line, "child-owned" | "child-owned lane")
        || has_present_child_owner_metadata(line)
        || field_value(line, "owner decision").is_some_and(is_child_delegation_owner_decision)
        || has_child_lane_owner_metadata(line)
}

fn has_complete_child_classification_before(lines: &[&str], end: usize) -> bool {
    let Some(start) = lines[..end]
        .iter()
        .rposition(|line| trimmed_value(line) == "| field | value |")
    else {
        return false;
    };
    if !lines
        .get(start + 1)
        .is_some_and(|line| is_table_separator(trimmed_value(line)))
        || lines[start + 2..end]
            .iter()
            .enumerate()
            .any(|(offset, line)| {
                is_lane_context_boundary(lines, start + offset + 2, trimmed_value(line))
            })
    {
        return false;
    }
    let mut fields = ClassificationFields::default();
    for (key, value) in lines[start + 2..end]
        .iter()
        .filter_map(|line| classification_table_row(trimmed_value(line)))
    {
        fields.record(metadata_key(key), trimmed_value(value));
    }
    fields.is_complete()
}

fn has_present_child_owner_metadata(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, value)| {
        metadata_key(key) == "child owner"
            && !trimmed_value(value).is_empty()
            && !has_absent_field_value(value, "child owner")
    })
}

fn is_parent_owned_lane_evidence(line: &str) -> bool {
    let line = metadata_key(line);
    if field_value(line, "owner decision").is_some_and(|value| {
        is_parent_owned_value(value) && !is_child_delegation_owner_decision(value)
    }) {
        return true;
    }
    has_parent_lane_owner_metadata(line)
}

fn has_child_lane_owner_metadata(line: &str) -> bool {
    ["lane ownership", "owner", "lane owner"]
        .into_iter()
        .any(|field| {
            field_value(line, field).is_some_and(|value| {
                matches!(trimmed_value(value), "child-owned" | "current-thread-owned")
            })
        })
}

fn has_parent_lane_owner_metadata(line: &str) -> bool {
    ["lane ownership", "owner", "lane owner"]
        .into_iter()
        .any(|field| {
            field_value(line, field).is_some_and(|value| trimmed_value(value) == "parent-owned")
        })
}

fn is_later_lane_boundary(lines: &[&str], index: usize, line: &str) -> bool {
    let line = metadata_key(line);
    is_parent_owned_lane_evidence(line)
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

fn is_inside_task_classification(lines: &[&str], index: usize) -> bool {
    for line in lines
        .iter()
        .take(index)
        .rev()
        .map(|line| trimmed_value(line))
    {
        if line.is_empty() {
            continue;
        }
        if metadata_key(line) == "task classification:" {
            return true;
        }
        if is_lane_boundary_terminator(line) || is_hard_lane_boundary(line) {
            return false;
        }
        if !is_task_classification_field(line) {
            return false;
        }
    }
    false
}

fn is_lane_boundary_terminator(line: &str) -> bool {
    "review response:|maintainer reassignment:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_hard_lane_boundary(line: &str) -> bool {
    "pr:|pull request:|lane ownership:"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn is_task_classification_field(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        matches!(
            metadata_key(key),
            "lane type"
                | "secondary surfaces"
                | "owner decision"
                | "atomic scope"
                | "required skills"
                | "required tools/evidence"
                | "required tools"
                | "required evidence"
                | "first allowed action"
                | "stop/blocker"
                | "stop blocker"
                | "blocker"
        )
    })
}
